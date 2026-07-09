#![allow(clippy::expect_used)]

use std::path::PathBuf;

use stp_core::ids::{TerminalId, WindowId, WorkspaceId};
use stp_core::registry::{ManagedTerminal, Registry, TerminalBackend, TerminalStatus};
use unicode_width::UnicodeWidthStr;

use super::session_sidebar::{
    WIDTH, command, mouse_binding, terminal_for_mouse_line, text,
};

#[test]
fn sidebar_rows_include_all_live_sessions_in_registry_order() {
    let registry = Registry {
        terminals: vec![
            terminal(
                "00000000-0000-0000-0000-000000000101",
                "/tmp/worktree-a",
                "main",
            ),
            terminal(
                "00000000-0000-0000-0000-000000000102",
                "/tmp/worktree-b",
                "feature/sidebar",
            ),
            terminal(
                "00000000-0000-0000-0000-000000000103",
                "/tmp/worktree-c",
                "feature/overflow",
            ),
        ],
    };

    let rendered = text(&registry);

    let row_a = rendered.find("1 00000000 worktree-a main").expect("row a");
    let row_b = rendered
        .find("2 00000000 worktree-b feature/sidebar")
        .expect("row b");
    let row_c = rendered
        .find("3 00000000 worktree-c feature/overflow")
        .expect("row c");
    assert!(row_a < row_b);
    assert!(row_b < row_c);
}

#[test]
fn sidebar_ignores_stale_and_exited_sessions() {
    let mut stale = terminal(
        "00000000-0000-0000-0000-000000000201",
        "/tmp/worktree-stale",
        "stale",
    );
    stale.status = TerminalStatus::Stale;
    let mut exited = terminal(
        "00000000-0000-0000-0000-000000000202",
        "/tmp/worktree-exited",
        "exited",
    );
    exited.status = TerminalStatus::Exited;
    let live = terminal(
        "00000000-0000-0000-0000-000000000203",
        "/tmp/worktree-live",
        "live",
    );
    let registry = Registry {
        terminals: vec![stale, live, exited],
    };

    let rendered = text(&registry);

    assert!(rendered.contains("1 00000000 worktree-live live"));
    assert!(!rendered.contains("worktree-stale"));
    assert!(!rendered.contains("worktree-exited"));
}

#[test]
fn command_renders_session_rows_when_live_sessions_exist() {
    let terminal_a = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/worktree-a",
        "main",
    );
    let terminal_b = terminal(
        "00000000-0000-0000-0000-000000000102",
        "/tmp/worktree-b",
        "feature/sidebar",
    );

    let registry = Registry {
        terminals: vec![terminal_a, terminal_b],
    };
    let rendered = command(&registry);

    assert!(rendered.contains("STP sessions"));
    assert!(rendered.contains("2 live sessions"));
    assert!(rendered.contains("Click row to focus/open"));
    assert!(rendered.contains("q quit panel"));
    assert!(rendered.contains("prefix K terminate focused"));
    assert!(rendered.contains("1 00000000 worktree-a main"));
    assert!(rendered.contains("2 00000000 worktree-b feature/sidebar"));
    assert!(rendered.contains(concat!("exec $", "{SHELL:-sh}")));
}

#[test]
fn sidebar_empty_state_shows_next_action() {
    let registry = Registry::default();

    let rendered = text(&registry);

    assert!(rendered.contains("0 live sessions"));
    assert!(rendered.contains("No live STP sessions"));
    assert!(rendered.contains("Open STP terminal in Cursor"));
}

#[test]
fn mouse_binding_focuses_existing_pane_or_respawns_empty_slot() {
    let binding = mouse_binding("/opt/stp/bin/stp", "/tmp/registry.json", "stp-managed");
    let joined_args = binding.args.join(" ");

    assert_eq!(binding.command, "if-shell");
    assert!(joined_args.contains("-t ="));
    assert!(joined_args.contains(concat!("#", "{==:#", "{@stp-pane-key},stp-sidebar}")));
    assert!(joined_args.contains("panel-select"));
    assert!(joined_args.contains("--registry"));
    assert!(joined_args.contains("--socket"));
    assert!(joined_args.contains(concat!("#", "{q:mouse_line}")));
    assert!(!joined_args.contains("stp-test-session"));
    assert!(!joined_args.contains("case \"$terminal_id\""));
}

#[test]
fn mouse_binding_command_size_is_constant() {
    let binding_a = mouse_binding("/opt/stp/bin/stp", "/tmp/registry-a.json", "stp-managed");
    let binding_b = mouse_binding("/opt/stp/bin/stp", "/tmp/registry-b.json", "stp-managed");

    assert_eq!(binding_a.args.len(), binding_b.args.len());
    assert!(binding_a.args.join(" ").contains("send-keys -M"));
    assert!(!binding_a.args.join(" ").contains("00000000-0000-0000"));
}

#[test]
fn command_replaces_control_characters_in_visible_labels() {
    let terminal = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/worktree\nbad",
        "main\tbad",
    );

    let registry = Registry {
        terminals: vec![terminal],
    };
    let rendered = command(&registry);

    assert!(rendered.contains("worktree?bad main?bad"));
}

#[test]
fn sidebar_rows_do_not_exceed_width() {
    let terminal = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/worktree-with-a-very-long-name-that-would-wrap",
        "feature/a-very-long-branch-name",
    );
    let registry = Registry {
        terminals: vec![terminal],
    };

    let rendered = text(&registry);

    for line in rendered.lines() {
        assert!(UnicodeWidthStr::width(line) <= WIDTH, "{line}");
    }
}

#[test]
fn sidebar_rows_with_wide_characters_do_not_exceed_width() {
    let terminal = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/터미널패널터미널패널터미널패널",
        "기능/사이드바-클릭-흐름",
    );
    let registry = Registry {
        terminals: vec![terminal],
    };

    let rendered = text(&registry);

    for line in rendered.lines() {
        assert!(UnicodeWidthStr::width(line) <= WIDTH, "{line}");
    }
}

#[test]
fn sidebar_rows_with_symbol_wide_characters_do_not_exceed_width() {
    let terminal = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/watch-watch-watch",
        "feature/⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚⌚",
    );
    let registry = Registry {
        terminals: vec![terminal],
    };

    let rendered = text(&registry);

    for line in rendered.lines() {
        assert!(UnicodeWidthStr::width(line) <= WIDTH, "{line}");
    }
}

#[test]
fn sidebar_mouse_line_maps_only_session_rows() {
    let terminal_a = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/worktree-a",
        "main",
    );
    let terminal_b = terminal(
        "00000000-0000-0000-0000-000000000102",
        "/tmp/worktree-b",
        "feature/sidebar",
    );
    let registry = Registry {
        terminals: vec![terminal_a, terminal_b],
    };

    let selected = terminal_for_mouse_line(&registry, "2 00000000 worktree-b feature/sidebar")
        .expect("terminal");

    assert_eq!(
        selected.terminal_id,
        TerminalId::parse("00000000-0000-0000-0000-000000000102").expect("id")
    );
    assert!(terminal_for_mouse_line(&registry, "STP sessions").is_none());
    assert!(terminal_for_mouse_line(&registry, "99 missing").is_none());
}

#[test]
fn sidebar_mouse_line_uses_tmux_coordinates_after_header() {
    let terminal_a = terminal(
        "00000000-0000-0000-0000-000000000101",
        "/tmp/worktree-a",
        "main",
    );
    let terminal_b = terminal(
        "00000000-0000-0000-0000-000000000102",
        "/tmp/worktree-b",
        "feature/sidebar",
    );
    let registry = Registry {
        terminals: vec![terminal_a, terminal_b],
    };

    assert!(terminal_for_mouse_line(&registry, "0").is_none());
    assert!(terminal_for_mouse_line(&registry, "1").is_none());
    assert!(terminal_for_mouse_line(&registry, "2").is_none());
    assert!(terminal_for_mouse_line(&registry, "3").is_none());
    assert!(terminal_for_mouse_line(&registry, "4").is_none());
    assert!(terminal_for_mouse_line(&registry, "5").is_none());
    assert!(terminal_for_mouse_line(&registry, "").is_none());
    assert!(terminal_for_mouse_line(&registry, "999999").is_none());
    let first_selected = terminal_for_mouse_line(&registry, "6").expect("first terminal");
    let selected = terminal_for_mouse_line(&registry, "7").expect("second terminal");

    assert_eq!(
        first_selected.terminal_id,
        TerminalId::parse("00000000-0000-0000-0000-000000000101").expect("id")
    );
    assert_eq!(
        selected.terminal_id,
        TerminalId::parse("00000000-0000-0000-0000-000000000102").expect("id")
    );
}

fn terminal(id: &str, workspace: &str, branch: &str) -> ManagedTerminal {
    ManagedTerminal {
        terminal_id: TerminalId::parse(id).expect("terminal id"),
        workspace_id: WorkspaceId::new("workspace".to_owned()),
        window_id: WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id"),
        workspace_path: PathBuf::from(workspace),
        repo_root: PathBuf::from(workspace),
        branch_name: Some(branch.to_owned()),
        backend: TerminalBackend::legacy_tmux(
            "stp-test-socket".to_owned(),
            "stp-test-session".to_owned(),
            "0".to_owned(),
        ),
        tmux_socket: "stp-test-socket".to_owned(),
        tmux_session: "stp-test-session".to_owned(),
        tmux_window: "0".to_owned(),
        created_at: 0,
        last_seen_at: 0,
        status: TerminalStatus::Live,
    }
}
