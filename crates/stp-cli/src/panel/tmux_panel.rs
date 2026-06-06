use std::path::Path;

use anyhow::{Context, bail};
use stp_core::registry::{ManagedTerminal, Registry, RegistryStore, TerminalStatus};
use stp_tmux::adapter::Tmux;

use super::Layout;
use crate::session_cleanup::mark_missing_live_sessions_stale;

const PANEL_SESSION: &str = "stp-panel";
const PANEL_WINDOW: &str = "panel";

pub fn open(store: &RegistryStore, layout: Layout, panel_socket: &str) -> anyhow::Result<()> {
    let mut registry = store.load()?;
    if mark_missing_live_sessions_stale(&mut registry) {
        store.save(&registry)?;
    }
    let commands = pane_commands(&registry, layout);
    let titles = pane_titles(&registry, layout);
    let first_command = commands.first().context("panel layout has no panes")?;
    let tmux = Tmux::new(panel_socket);
    tmux.kill_session_if_exists(PANEL_SESSION)?;
    tmux.new_session_with_window(PANEL_SESSION, PANEL_WINDOW, first_command)?;
    tmux.set_option(PANEL_SESSION, "status", "off")?;
    tmux.set_option(PANEL_SESSION, "mouse", "on")?;
    install_terminate_binding(&tmux, store.path())?;
    for command in commands.iter().skip(1) {
        tmux.split_window(PANEL_SESSION, command)?;
        tmux.select_tiled_layout(PANEL_SESSION)?;
    }
    tmux.select_tiled_layout(PANEL_SESSION)?;
    let pane_ids = tmux.list_pane_ids(PANEL_SESSION)?;
    if pane_ids.len() != titles.len() {
        bail!(
            "panel pane count mismatch: expected {}, got {}",
            titles.len(),
            pane_ids.len()
        );
    }
    for (pane_id, title) in pane_ids.iter().zip(titles.iter()) {
        tmux.set_pane_title(pane_id, title)?;
    }
    tmux.attach_session(PANEL_SESSION)?;
    Ok(())
}

fn pane_commands(registry: &Registry, layout: Layout) -> Vec<String> {
    let capacity = layout.capacity();
    let terminals = panel_terminals(registry);
    (0..capacity)
        .map(|slot| {
            terminals
                .get(slot)
                .copied()
                .map_or_else(|| empty_command(slot), terminal_command)
        })
        .collect()
}

fn pane_titles(registry: &Registry, layout: Layout) -> Vec<String> {
    let capacity = layout.capacity();
    let terminals = panel_terminals(registry);
    (0..capacity)
        .map(|slot| {
            terminals.get(slot).map_or_else(
                || format!("empty:{}", slot.saturating_add(1)),
                |terminal| terminal.terminal_id.to_string(),
            )
        })
        .collect()
}

fn panel_terminals(registry: &Registry) -> Vec<&ManagedTerminal> {
    registry
        .terminals
        .iter()
        .filter(|terminal| terminal.status == TerminalStatus::Live)
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

fn install_terminate_binding(tmux: &Tmux, registry_path: &Path) -> anyhow::Result<()> {
    let binary = std::env::current_exe().context("failed to resolve stp binary path")?;
    let binding = terminate_binding(&binary, registry_path);
    tmux.bind_key_args(
        "K",
        "confirm-before",
        &["-p", &binding.prompt, &binding.run_command],
    )?;
    Ok(())
}

#[derive(Debug)]
struct TerminateBinding {
    prompt: String,
    run_command: String,
}

fn terminate_binding(binary: &Path, registry_path: &Path) -> TerminateBinding {
    let terminate_cli = format!(
        "{} terminate --registry {}",
        shell_double_quote(&binary.display().to_string()),
        shell_double_quote(&registry_path.display().to_string()),
    );
    let shell_command = format!(
        "terminal_id=#{{q:pane_title}}; case \"$terminal_id\" in empty:*|\"\") tmux display-message 'No selected STP terminal';; *) {terminate_cli} --terminal-id \"$terminal_id\" --yes;; esac",
    );
    TerminateBinding {
        prompt: "Terminate STP #{pane_title}? (y/n)".to_owned(),
        run_command: format!("run-shell -b {}", shell_quote(&shell_command)),
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_double_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`")
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use stp_core::ids::{TerminalId, WindowId, WorkspaceId};
    use stp_core::registry::{ManagedTerminal, Registry, TerminalStatus};

    use super::{pane_commands, pane_titles, terminate_binding};
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

    #[test]
    fn pane_titles_use_terminal_ids_and_empty_slot_titles() {
        let registry = Registry {
            terminals: vec![terminal("00000000-0000-0000-0000-000000000101")],
        };

        let titles = pane_titles(&registry, Layout::TwoByTwo);

        assert_eq!(titles[0], "00000000-0000-0000-0000-000000000101");
        assert_eq!(titles[1], "empty:2");
    }

    #[test]
    fn pane_commands_ignore_exited_terminal_sessions() {
        let registry = Registry {
            terminals: vec![
                terminal_with_status(
                    "00000000-0000-0000-0000-000000000101",
                    TerminalStatus::Exited,
                ),
                terminal("00000000-0000-0000-0000-000000000102"),
            ],
        };

        let commands = pane_commands(&registry, Layout::TwoByTwo);
        let titles = pane_titles(&registry, Layout::TwoByTwo);

        assert!(commands[0].contains("attach-session -t 'stp-test-session'"));
        assert!(commands[1].contains("slot 2: <empty>"));
        assert_eq!(titles[0], "00000000-0000-0000-0000-000000000102");
        assert_eq!(titles[1], "empty:2");
    }

    #[test]
    fn terminate_binding_uses_prefix_safe_cli_command() {
        let binding = terminate_binding(
            &PathBuf::from("/opt/stp/bin/stp"),
            &PathBuf::from("/tmp/registry.json"),
        );

        assert!(binding.prompt.contains("#{pane_title}"));
        assert!(binding.run_command.contains("run-shell -b"));
        assert!(binding.run_command.contains("terminate --registry"));
        assert!(binding.run_command.contains("/tmp/registry.json"));
        assert!(
            binding
                .run_command
                .contains(concat!("terminal_id=", "#{q:pane_title}"))
        );
        assert!(
            binding
                .run_command
                .contains("--terminal-id \"$terminal_id\" --yes")
        );
        assert!(binding.run_command.contains("No selected STP terminal"));
    }

    fn terminal(id: &str) -> ManagedTerminal {
        terminal_with_status(id, TerminalStatus::Live)
    }

    fn terminal_with_status(id: &str, status: TerminalStatus) -> ManagedTerminal {
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
            status,
        }
    }
}
