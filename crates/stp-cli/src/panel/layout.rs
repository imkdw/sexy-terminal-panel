use stp_core::registry::{ManagedTerminal, Registry, TerminalStatus};

use super::Layout;
use super::shell;

pub(super) fn pane_commands(registry: &Registry, layout: Layout) -> Vec<String> {
    let terminals = panel_terminals(registry);
    pane_commands_for_terminals(&terminals, layout)
}

pub(super) fn pane_titles(registry: &Registry, layout: Layout) -> Vec<String> {
    let terminals = panel_terminals(registry);
    pane_titles_for_terminals(&terminals, layout)
}

fn pane_commands_for_terminals(terminals: &[&ManagedTerminal], layout: Layout) -> Vec<String> {
    (0..layout.capacity())
        .map(|slot| {
            terminals
                .get(slot)
                .copied()
                .map_or_else(|| empty_command(slot), terminal_command)
        })
        .collect()
}

fn pane_titles_for_terminals(terminals: &[&ManagedTerminal], layout: Layout) -> Vec<String> {
    (0..layout.capacity())
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
        shell::quote(&terminal.tmux_socket),
        shell::quote(&terminal.tmux_session),
    )
}

fn empty_command(slot: usize) -> String {
    let message = format!(
        "slot {}: <empty>\n\nOpen a new STP terminal in Cursor, then run stp panel again.\n",
        slot.saturating_add(1)
    );
    format!("printf %s {}; exec ${{SHELL:-sh}}", shell::quote(&message))
}
