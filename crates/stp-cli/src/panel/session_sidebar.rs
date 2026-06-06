use stp_core::registry::{ManagedTerminal, Registry, TerminalStatus};

pub(super) const TITLE: &str = "stp-sidebar";
pub(super) const WIDTH: usize = 44;
const HEADER_LINES: usize = 3;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct MouseBinding {
    pub command: &'static str,
    pub args: Vec<String>,
}

pub(super) fn command(registry: &Registry) -> String {
    format!(
        "printf %s {}; exec ${{SHELL:-sh}}",
        shell_quote(&text(registry))
    )
}

pub(super) fn mouse_binding(binary: &str, registry_path: &str, socket: &str) -> MouseBinding {
    let select_command = format!(
        "{} panel-select --registry {} --socket {} --mouse-line \"#{{q:mouse_line}}\"",
        shell_double_quote(binary),
        shell_double_quote(registry_path),
        shell_double_quote(socket),
    );
    MouseBinding {
        command: "if-shell",
        args: vec![
            "-b".to_owned(),
            "-F".to_owned(),
            "-t".to_owned(),
            "=".to_owned(),
            format!("#{{==:#{{@stp-pane-key}},{TITLE}}}"),
            format!("run-shell -b {}", shell_quote(&select_command)),
            "select-pane -t = ; send-keys -M".to_owned(),
        ],
    }
}

pub(super) fn text(registry: &Registry) -> String {
    let terminals = live_terminals(registry);
    let mut output = String::from("STP sessions\nClick a session\n\n");
    if terminals.is_empty() {
        output.push_str("No live STP sessions\n");
        return output;
    }
    for (index, terminal) in terminals.iter().enumerate() {
        let slot = index.saturating_add(1);
        let workspace = terminal
            .workspace_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("<workspace>");
        let branch = terminal.branch_name.as_deref().unwrap_or("non-git");
        let line = format!(
            "{slot} {} {} {}",
            short_terminal_id(terminal),
            display_text(workspace),
            display_text(branch)
        );
        output.push_str(&fit_line(&line));
        output.push('\n');
    }
    output
}

pub(super) fn terminal_for_mouse_line(
    registry: &Registry,
    mouse_line: &str,
) -> Option<ManagedTerminal> {
    let trimmed = mouse_line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        let line = trimmed.parse::<usize>().ok()?;
        return line
            .checked_sub(HEADER_LINES)
            .and_then(|index| live_terminals(registry).get(index).cloned());
    }
    let slot = trimmed
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<usize>().ok())?;
    if slot == 0 {
        return None;
    }
    live_terminals(registry)
        .get(slot.saturating_sub(1))
        .cloned()
}

pub(super) fn live_terminals(registry: &Registry) -> Vec<ManagedTerminal> {
    registry
        .terminals
        .iter()
        .filter(|terminal| terminal.status == TerminalStatus::Live)
        .cloned()
        .collect()
}

pub(super) fn terminal_command(terminal: &ManagedTerminal) -> String {
    format!(
        "env -u TMUX tmux -L {} attach-session -t {} || exec ${{SHELL:-sh}}",
        shell_quote(&terminal.tmux_socket),
        shell_quote(&terminal.tmux_session),
    )
}

fn short_terminal_id(terminal: &ManagedTerminal) -> String {
    terminal.terminal_id.to_string().chars().take(8).collect()
}

fn display_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect()
}

fn fit_line(value: &str) -> String {
    let char_count = value.chars().count();
    if char_count <= WIDTH {
        return value.to_owned();
    }
    if WIDTH <= 3 {
        return ".".repeat(WIDTH);
    }
    let prefix: String = value.chars().take(WIDTH.saturating_sub(3)).collect();
    format!("{prefix}...")
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
