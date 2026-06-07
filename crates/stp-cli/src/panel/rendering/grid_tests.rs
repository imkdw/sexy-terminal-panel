use std::path::PathBuf;

use stp_core::ids::{TerminalId, WindowId, WorkspaceId};
use stp_core::registry::{ManagedTerminal, Registry, TerminalBackend, TerminalStatus};

use super::grid::{LineEnding, display_text, render, truncate_to_width};
use super::render_once;
use crate::panel::Layout;

#[test]
#[allow(clippy::expect_used)]
fn render_uses_grid_columns_for_slots() {
    let registry = Registry {
        terminals: vec![terminal(
            "00000000-0000-0000-0000-000000000101",
            "one",
            "main",
        )],
    };
    let mut buffer = Vec::new();

    render(
        &registry,
        Layout::ThreeByThree,
        0,
        LineEnding::Lf,
        Some(72),
        &mut buffer,
    )
    .expect("render");

    let rendered = String::from_utf8(buffer).expect("utf8 render");
    assert!(
        rendered.contains("+----------------------+----------------------+----------------------+")
    );
    assert!(rendered.contains("|>1: one"));
    assert!(rendered.contains("| 2: <empty>"));
    assert!(rendered.contains("| 3: <empty>"));
}

#[test]
#[allow(clippy::expect_used)]
fn render_once_shows_session_list_and_right_grid_preview() {
    let registry = Registry {
        terminals: vec![terminal(
            "00000000-0000-0000-0000-000000000101",
            "worktree-a",
            "feature/sidebar",
        )],
    };

    let rendered = render_once(&registry, Layout::ThreeByThree).expect("render");

    assert!(rendered.contains("STP sessions"));
    assert!(rendered.contains("Click a session"));
    assert!(rendered.contains("1 00000000 worktree-a feature/sidebar"));
    assert!(rendered.contains("STP panel\nLayout: 3x3 | Focus slot: 1\n"));
    assert!(rendered.contains("|>1: worktree-a"));
}

#[test]
#[allow(clippy::expect_used)]
fn render_ignores_non_live_sessions_in_grid_preview() {
    let registry = Registry {
        terminals: vec![
            terminal_with_status(
                "00000000-0000-0000-0000-000000000101",
                "stale-worktree",
                "main",
                TerminalStatus::Stale,
            ),
            terminal(
                "00000000-0000-0000-0000-000000000102",
                "live-worktree",
                "main",
            ),
        ],
    };
    let mut buffer = Vec::new();

    render(
        &registry,
        Layout::TwoByTwo,
        0,
        LineEnding::Lf,
        Some(120),
        &mut buffer,
    )
    .expect("render");

    let rendered = String::from_utf8(buffer).expect("utf8 render");
    assert!(!rendered.contains("stale-worktree"));
    assert!(rendered.contains("|>1: live-worktree"));
}

#[test]
fn truncate_to_width_keeps_line_inside_width() {
    let truncated = truncate_to_width("slot 1: abcdefghijklmnopqrstuvwxyz", 16);

    assert_eq!(truncated, "slot 1: abcde...");
    assert_eq!(truncated.chars().count(), 16);
}

#[test]
#[allow(clippy::expect_used)]
fn render_uses_default_width_when_width_is_zero() {
    let registry = Registry::default();
    let mut buffer = Vec::new();

    render(
        &registry,
        Layout::ThreeByThree,
        0,
        LineEnding::Lf,
        Some(0),
        &mut buffer,
    )
    .expect("render");

    let rendered = String::from_utf8(buffer).expect("utf8 render");
    assert!(rendered.contains("STP panel\nLayout: 3x3 | Focus slot: 1\n"));
}

#[test]
fn display_text_replaces_terminal_control_characters() {
    assert_eq!(display_text("main\u{1b}]0;owned\u{7}"), "main?]0;owned?");
}

#[test]
#[allow(clippy::expect_used)]
fn render_sanitizes_registry_display_fields() {
    let registry = Registry {
        terminals: vec![terminal(
            "00000000-0000-0000-0000-000000000101",
            "safe-workspace",
            "main\u{1b}[31m",
        )],
    };
    let mut buffer = Vec::new();

    render(
        &registry,
        Layout::ThreeByThree,
        0,
        LineEnding::Lf,
        Some(120),
        &mut buffer,
    )
    .expect("render");

    let rendered = String::from_utf8(buffer).expect("utf8 render");
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("main?[31m"));
}

#[allow(clippy::expect_used)]
fn terminal(id: &str, workspace: &str, branch: &str) -> ManagedTerminal {
    terminal_with_status(id, workspace, branch, TerminalStatus::Live)
}

#[allow(clippy::expect_used)]
fn terminal_with_status(
    id: &str,
    workspace: &str,
    branch: &str,
    status: TerminalStatus,
) -> ManagedTerminal {
    ManagedTerminal {
        terminal_id: TerminalId::parse(id).expect("terminal id"),
        workspace_id: WorkspaceId::new(format!("workspace-{workspace}")),
        window_id: WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id"),
        workspace_path: PathBuf::from(workspace),
        repo_root: PathBuf::from(workspace),
        branch_name: Some(branch.to_owned()),
        backend: TerminalBackend::legacy_tmux(
            "stp-test".to_owned(),
            "stp-test-session".to_owned(),
            "0".to_owned(),
        ),
        tmux_socket: "stp-test".to_owned(),
        tmux_session: "stp-test-session".to_owned(),
        tmux_window: "0".to_owned(),
        created_at: 0,
        last_seen_at: 0,
        status,
    }
}
