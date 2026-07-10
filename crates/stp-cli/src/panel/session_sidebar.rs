use stp_core::registry::{ManagedTerminal, Registry, TerminalStatus};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(super) const TITLE: &str = "stp-sidebar";
pub(super) const WIDTH: usize = 30;
const HEADER_LINES: usize = 6;

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
    let mut output = format!(
        concat!(
            "STP sessions\n",
            "{}\n",
            "click row open\n",
            "q quit panel\n",
            "prefix+K kill\n\n"
        ),
        live_count_label(terminals.len())
    );
    if terminals.is_empty() {
        output.push_str("No live STP sessions\n");
        output.push_str("Open STP terminal in Cursor\n");
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

fn live_count_label(count: usize) -> String {
    match count {
        1 => "1 live session".to_owned(),
        _ => format!("{count} live sessions"),
    }
}

fn display_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect()
}

fn fit_line(value: &str) -> String {
    if terminal_display_width(value) <= WIDTH {
        return value.to_owned();
    }
    if WIDTH <= 3 {
        return ".".repeat(WIDTH);
    }
    let target_width = WIDTH.saturating_sub(3);
    let mut current_width: usize = 0;
    let mut prefix = String::new();
    for ch in value.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width.saturating_add(char_width) > target_width {
            break;
        }
        current_width = current_width.saturating_add(char_width);
        prefix.push(ch);
    }
    format!("{prefix}...")
}

fn terminal_display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
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
