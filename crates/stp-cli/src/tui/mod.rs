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
use stp_core::registry::{RegistryStore, TerminalStatus};

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
    let result = event_loop(&mut terminal, &mut link, &mut state, &registry_path);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    let _ = ratatui::try_restore();
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

/// 시작 시 broker 가 아는 라이브 세션을 사이드바에 채우고 event 연결에 Attach.
fn seed_sessions(
    link: &mut BrokerLink,
    state: &mut PanelState,
    registry_path: &std::path::Path,
) -> Result<()> {
    let summaries = link.list_sessions()?;
    let store = RegistryStore::new(registry_path.to_path_buf());
    let registry = store.load().ok();
    for summary in summaries {
        let entry = registry
            .as_ref()
            .and_then(|reg| entry_from_registry(reg, &summary.terminal_id))
            .unwrap_or_else(|| fallback_entry(&summary.terminal_id));
        link.attach(&entry.id)?;
        state.add_session(entry);
    }
    Ok(())
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    link: &mut BrokerLink,
    state: &mut PanelState,
    registry_path: &std::path::Path,
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
            if apply_action(action, link, state, registry_path, last_mouse)? {
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
    registry_path: &std::path::Path,
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
        Action::Spawn => spawn_session(link, state, registry_path)?,
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

fn spawn_session(
    link: &mut BrokerLink,
    state: &mut PanelState,
    registry_path: &std::path::Path,
) -> Result<()> {
    let terminal_id = TerminalId::from_uuid(uuid::Uuid::new_v4());
    let window_id = WindowId::from_uuid(uuid::Uuid::new_v4());
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    link.spawn(&terminal_id, &window_id, workspace, None)?;
    let store = RegistryStore::new(registry_path.to_path_buf());
    let entry = store
        .load()
        .ok()
        .and_then(|reg| entry_from_registry(&reg, &terminal_id))
        .unwrap_or_else(|| fallback_entry(&terminal_id));
    link.attach(&terminal_id)?;
    state.add_session(entry);
    Ok(())
}

fn entry_from_registry(
    registry: &stp_core::registry::Registry,
    id: &TerminalId,
) -> Option<SessionEntry> {
    let terminal = registry
        .terminals
        .iter()
        .find(|terminal| terminal.terminal_id == *id && terminal.status != TerminalStatus::Exited)?;
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
    Some(SessionEntry {
        id: id.clone(),
        workspace,
        branch,
    })
}

fn fallback_entry(id: &TerminalId) -> SessionEntry {
    SessionEntry {
        id: id.clone(),
        workspace: id.to_string().chars().take(8).collect(),
        branch: "?".to_owned(),
    }
}

