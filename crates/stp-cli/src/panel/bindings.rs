use std::path::Path;

use anyhow::Context;
use stp_tmux::adapter::{BindingCommand, Tmux};

use super::session_sidebar;
use super::shell;

#[derive(Debug)]
pub(super) struct TerminateBinding {
    pub(super) prompt: String,
    pub(super) run_command: String,
}

pub(super) fn install_quit_binding(tmux: &Tmux) -> anyhow::Result<()> {
    let command = BindingCommand::new("detach-client");
    tmux.bind_key_in_table("root", "q", &command)?;
    Ok(())
}

pub(super) fn install_terminate_binding(
    tmux: &Tmux,
    registry_path: &Path,
    panel_socket: &str,
) -> anyhow::Result<()> {
    let binary = std::env::current_exe().context("failed to resolve stp binary path")?;
    let binding = terminate_binding(&binary, registry_path, panel_socket);
    let command = BindingCommand::confirm_before(&binding.prompt, &binding.run_command);
    tmux.bind_key_command("K", &command)?;
    Ok(())
}

pub(super) fn install_mouse_binding(
    tmux: &Tmux,
    registry_path: &Path,
    panel_socket: &str,
) -> anyhow::Result<()> {
    let binary = std::env::current_exe().context("failed to resolve stp binary path")?;
    let binding = session_sidebar::mouse_binding(
        &binary.display().to_string(),
        &registry_path.display().to_string(),
        panel_socket,
    );
    let command = binding
        .args
        .iter()
        .fold(BindingCommand::new(binding.command), |command, argument| {
            command.arg(argument)
        });
    tmux.bind_key_in_table("root", "MouseDown1Pane", &command)?;
    Ok(())
}

pub(super) fn terminate_binding(
    binary: &Path,
    registry_path: &Path,
    panel_socket: &str,
) -> TerminateBinding {
    let terminate_cli = format!(
        "{} terminate --registry {}",
        shell::double_quote(&binary.display().to_string()),
        shell::double_quote(&registry_path.display().to_string()),
    );
    let panel_tmux = format!("tmux -L {}", shell::quote(panel_socket));
    let shell_command = format!(
        "pane_id=#{{q:pane_id}}; terminal_id=$({panel_tmux} display-message -p -t \"$pane_id\" '##{{@stp-pane-key}}'); case \"$terminal_id\" in empty:*|stp-sidebar|\"\") {panel_tmux} display-message 'No selected STP terminal';; *) {terminate_cli} --terminal-id \"$terminal_id\" --yes;; esac",
    );
    TerminateBinding {
        prompt: "Terminate selected STP pane? (y/n)".to_owned(),
        run_command: format!("run-shell -b {}", shell::quote(&shell_command)),
    }
}
