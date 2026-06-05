use anyhow::Context;
use stp_core::registry::{ManagedTerminal, Registry, RegistryStore};
use stp_tmux::adapter::Tmux;

use super::Layout;

const PANEL_SESSION: &str = "stp-panel";
const PANEL_WINDOW: &str = "panel";

pub fn open(store: &RegistryStore, layout: Layout, panel_socket: &str) -> anyhow::Result<()> {
    let registry = store.load()?;
    let commands = pane_commands(&registry, layout);
    let first_command = commands.first().context("panel layout has no panes")?;
    let tmux = Tmux::new(panel_socket);
    tmux.kill_session_if_exists(PANEL_SESSION)?;
    tmux.new_session_with_window(PANEL_SESSION, PANEL_WINDOW, first_command)?;
    tmux.set_option(PANEL_SESSION, "status", "off")?;
    tmux.set_option(PANEL_SESSION, "mouse", "on")?;
    for command in commands.iter().skip(1) {
        tmux.split_window(PANEL_SESSION, command)?;
        tmux.select_tiled_layout(PANEL_SESSION)?;
    }
    tmux.select_tiled_layout(PANEL_SESSION)?;
    tmux.attach_session(PANEL_SESSION)?;
    Ok(())
}

fn pane_commands(registry: &Registry, layout: Layout) -> Vec<String> {
    let capacity = layout.capacity();
    (0..capacity)
        .map(|slot| {
            registry
                .terminals
                .get(slot)
                .map_or_else(|| empty_command(slot), terminal_command)
        })
        .collect()
}

fn terminal_command(terminal: &ManagedTerminal) -> String {
    format!(
        "env -u TMUX tmux -L {} attach-session -t {} || exec ${{SHELL:-sh}}",
        shell_quote(&terminal.tmux_socket),
        shell_quote(&terminal.tmux_session),
    )
}

fn empty_command(slot: usize) -> String {
    let message = format!(
        "slot {}: <empty>\n\nOpen a new STP terminal in VS Code, then run stp panel again.\n",
        slot.saturating_add(1)
    );
    format!("printf %s {}; exec ${{SHELL:-sh}}", shell_quote(&message))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use stp_core::ids::{TerminalId, WindowId, WorkspaceId};
    use stp_core::registry::{ManagedTerminal, Registry, TerminalStatus};

    use super::pane_commands;
    use crate::panel::Layout;

    #[test]
    fn pane_commands_attach_registered_terminal_sessions() {
        let registry = Registry {
            terminals: vec![terminal("00000000-0000-0000-0000-000000000101")],
        };

        let commands = pane_commands(&registry, Layout::ThreeByThree);

        assert_eq!(commands.len(), 9);
        assert!(commands[0].contains("env -u TMUX tmux -L 'stp-test-socket'"));
        assert!(commands[0].contains("attach-session -t 'stp-test-session'"));
        assert!(commands[1].contains("slot 2: <empty>"));
    }

    #[test]
    fn pane_commands_follow_two_by_two_capacity() {
        let registry = Registry::default();

        let commands = pane_commands(&registry, Layout::TwoByTwo);

        assert_eq!(commands.len(), 4);
        assert!(commands[3].contains("slot 4: <empty>"));
    }

    fn terminal(id: &str) -> ManagedTerminal {
        ManagedTerminal {
            terminal_id: TerminalId::parse(id).expect("terminal id"),
            workspace_id: WorkspaceId::new("workspace".to_owned()),
            window_id: WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id"),
            workspace_path: PathBuf::from("/tmp/workspace"),
            repo_root: PathBuf::from("/tmp/workspace"),
            branch_name: Some("main".to_owned()),
            tmux_socket: "stp-test-socket".to_owned(),
            tmux_session: "stp-test-session".to_owned(),
            tmux_window: "0".to_owned(),
            created_at: 0,
            last_seen_at: 0,
            status: TerminalStatus::Live,
        }
    }
}
