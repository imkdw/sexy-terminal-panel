//! 네이티브 ratatui 패널. broker(PTY 멀티플렉서) 위에 렌더/마우스를 직접 소유한다.

mod broker_link;
mod input;
mod layout;
mod render;
mod state;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, MouseEvent,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
use stp_core::ids::{TerminalId, WindowId};
use stp_core::registry::{ManagedTerminal, RegistryStore, TerminalBackend, TerminalStatus};

use crate::cli::TuiArgs;
use crate::state::{selected_broker_socket_path, selected_registry_path};
use broker_link::{BrokerLink, LinkEvent};
use input::Action;
use layout::pty_size;
use state::{Drag, GridKind, PanelState, SessionEntry};

const READY_TIMEOUT: Duration = Duration::from_secs(3);
/// 렌더 상한. Output 코얼레싱 창이기도 하다.
const TICK: Duration = Duration::from_millis(33);

pub fn run(args: &TuiArgs) -> Result<()> {
    let registry_path = selected_registry_path(args.registry.clone());
    let socket = selected_broker_socket_path(args.broker_socket.clone());
    ensure_broker(&registry_path, &socket)?;
    let mut link = BrokerLink::connect(&socket)?;
    let mut state = PanelState::new(GridKind::TwoByTwo);
    seed_sessions(&mut link, &mut state, &registry_path)?;

    let mut terminal = ratatui::try_init().context("failed to enter raw mode")?;
    let _ = execute!(io::stdout(), EnableMouseCapture);
    let result = event_loop(&mut terminal, &mut link, &mut state);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    let _ = ratatui::try_restore();
    // 패널이 띄운 broker PTY 정리. 브리지는 tmux 세션을 detach 만 하고(세션 생존),
    // + new 로 만든 셸은 종료된다.
    for entry in &state.sessions {
        let _ = link.terminate(&entry.id);
    }
    result
}

fn ensure_broker(registry_path: &std::path::Path, socket: &std::path::Path) -> Result<()> {
    let config = stp_pty::BrokerConfig::new(
        registry_path.to_path_buf(),
        socket.to_path_buf(),
        READY_TIMEOUT,
    );
    let executable = std::env::current_exe().context("failed to locate stp executable")?;
    stp_pty::ensure_broker(&config, &executable).context("failed to ensure broker")?;
    Ok(())
}

/// 시작 시 registry 의 live tmux 세션을 각각 broker PTY(`tmux attach` 래핑)로 브리지해
/// 사이드바에 채우고 event 연결에 Attach. broker 자체 PTY 세션이 아니라 tmux 가 진실원.
fn seed_sessions(
    link: &mut BrokerLink,
    state: &mut PanelState,
    registry_path: &std::path::Path,
) -> Result<()> {
    let store = RegistryStore::new(registry_path.to_path_buf());
    let Ok(registry) = store.load() else {
        return Ok(());
    };
    for terminal in &registry.terminals {
        if terminal.status != TerminalStatus::Live {
            continue;
        }
        let TerminalBackend::LegacyTmux { socket, session, .. } = &terminal.backend else {
            continue; // Pty 백엔드(브리지 잔여 등)는 건너뜀
        };
        let bridge_id = TerminalId::from_uuid(uuid::Uuid::new_v4());
        let window_id = WindowId::from_uuid(uuid::Uuid::new_v4());
        let argv = tmux_attach_argv(socket, session);
        if link
            .spawn(
                &bridge_id,
                &window_id,
                terminal.workspace_path.clone(),
                None,
                Some(argv),
            )
            .is_err()
        {
            continue;
        }
        link.attach(&bridge_id)?;
        let (workspace, branch) = labels(terminal);
        state.add_session(SessionEntry {
            id: bridge_id,
            workspace,
            branch,
        });
    }
    Ok(())
}

/// 기존 tmux 세션에 붙는 PTY 명령. `|| exec` 로 tmux 종료 시 셸로 폴백.
fn tmux_attach_argv(socket: &str, session: &str) -> Vec<String> {
    vec![
        "sh".to_owned(),
        "-c".to_owned(),
        format!(
            "env -u TMUX tmux -L {socket} attach-session -t {session} || exec ${{SHELL:-sh}}"
        ),
    ]
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    link: &mut BrokerLink,
    state: &mut PanelState,
) -> Result<()> {
    let mut resized: HashMap<TerminalId, (u16, u16)> = HashMap::new();
    let mut last_mouse = (0_u16, 0_u16);
    loop {
        for link_event in link.drain() {
            match link_event {
                LinkEvent::Exited(id) => {
                    state.remove_session(&id);
                    resized.remove(&id);
                }
                LinkEvent::Disconnected => return Ok(()),
                LinkEvent::Output(_) | LinkEvent::Snapshot(_) => {}
            }
        }

        let area = terminal_area(terminal)?;
        let regions = layout::compute(area, state);
        reconcile_resizes(link, state, &regions, &mut resized);

        terminal
            .draw(|frame| render::draw(frame, state, &regions, link))
            .context("draw frame")?;

        if event::poll(TICK)? {
            let event = event::read()?;
            if let Event::Mouse(MouseEvent { column, row, .. }) = &event {
                last_mouse = (*column, *row);
            }
            let action = input::map(&event, &regions, state);
            if apply_action(action, link, state, last_mouse)? {
                return Ok(());
            }
        }
    }
}

fn terminal_area(terminal: &ratatui::DefaultTerminal) -> Result<Rect> {
    let size = terminal.size().context("terminal size")?;
    Ok(Rect::new(0, 0, size.width, size.height))
}

/// 배치된 각 pane 을 타일 본문 크기에 맞춰 Resize(변경 시에만). 그리드 토글/창 리사이즈/swap 을 모두 커버.
fn reconcile_resizes(
    link: &mut BrokerLink,
    state: &PanelState,
    regions: &layout::Regions,
    resized: &mut HashMap<TerminalId, (u16, u16)>,
) {
    for (slot, tile) in regions.tiles.iter().enumerate() {
        let Some(id) = state.slots.get(slot).and_then(Option::as_ref) else {
            continue;
        };
        let (cols, rows) = pty_size(tile.body);
        if resized.get(id) == Some(&(cols, rows)) {
            continue;
        }
        if link.resize(id, cols, rows).is_ok() {
            resized.insert(id.clone(), (cols, rows));
        }
    }
}

/// 액션 적용. 종료해야 하면 true.
fn apply_action(
    action: Action,
    link: &mut BrokerLink,
    state: &mut PanelState,
    last_mouse: (u16, u16),
) -> Result<bool> {
    match action {
        Action::Quit => return Ok(true),
        Action::None => {}
        Action::FocusSlot(slot) => state.focus_slot(slot),
        Action::PlaceSession(index) => {
            state.place_session(index);
        }
        Action::CloseSession(index) => {
            // ponytail: 확인 대화 없이 즉시 종료. 필요하면 확인 단계 추가.
            if let Some(entry) = state.sessions.get(index) {
                let id = entry.id.clone();
                let _ = link.terminate(&id);
                state.remove_session(&id);
            }
        }
        Action::Spawn => spawn_session(link, state)?,
        Action::SetGrid(kind) => state.set_grid(kind),
        Action::DragStart(slot) => {
            state.drag = Some(Drag {
                from_slot: slot,
                cursor: last_mouse,
            });
        }
        Action::DragMove(x, y) => {
            if let Some(drag) = state.drag.as_mut() {
                drag.cursor = (x, y);
            }
        }
        Action::DragEnd(target) => {
            if let Some(drag) = state.drag.take()
                && let Some(target) = target
            {
                state.swap_slots(drag.from_slot, target);
            }
        }
        Action::Key(bytes) => {
            if let Some(id) = state.focused_terminal() {
                let _ = link.input(&id.clone(), &bytes);
            }
        }
    }
    Ok(false)
}

fn spawn_session(link: &mut BrokerLink, state: &mut PanelState) -> Result<()> {
    let terminal_id = TerminalId::from_uuid(uuid::Uuid::new_v4());
    let window_id = WindowId::from_uuid(uuid::Uuid::new_v4());
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace_name = workspace
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_owned();
    link.spawn(&terminal_id, &window_id, workspace, None, None)?;
    link.attach(&terminal_id)?;
    state.add_session(SessionEntry {
        id: terminal_id,
        workspace: workspace_name,
        branch: "shell".to_owned(),
    });
    Ok(())
}

/// `ManagedTerminal` 에서 사이드바 라벨(workspace, branch) 추출.
fn labels(terminal: &ManagedTerminal) -> (String, String) {
    let workspace = terminal
        .workspace_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_owned();
    let branch = terminal
        .branch_name
        .clone()
        .unwrap_or_else(|| "non-git".to_owned());
    (workspace, branch)
}

