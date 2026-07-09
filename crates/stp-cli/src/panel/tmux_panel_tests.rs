#![allow(clippy::expect_used)]

use std::path::PathBuf;

use stp_core::ids::{TerminalId, WindowId, WorkspaceId};
use stp_core::registry::{ManagedTerminal, Registry, TerminalBackend, TerminalStatus};
use stp_tmux::adapter::Tmux;

use super::Layout;
use super::bindings::install_quit_binding;
use super::tmux_panel::{pane_commands, pane_titles, terminate_binding};

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
    assert!(commands[1].contains("click it in the sidebar"));
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
        "stp-managed",
    );

    assert!(binding.prompt.contains("selected STP pane"));
    assert!(binding.run_command.contains("run-shell -b"));
    assert!(!binding.run_command.contains("run-shell -b -F"));
    assert!(binding.run_command.contains("terminate --registry"));
    assert!(binding.run_command.contains("/tmp/registry.json"));
    assert!(
        binding
            .run_command
            .contains(concat!("pane_id=", "#{q:pane_id}"))
    );
    assert!(binding.run_command.contains("##{@stp-pane-key}"));
    assert!(binding.run_command.contains("tmux -L"));
    assert!(binding.run_command.contains("stp-managed"));
    assert!(
        binding
            .run_command
            .contains("--terminal-id \"$terminal_id\" --yes")
    );
    assert!(binding.run_command.contains("No selected STP terminal"));
    assert!(binding.run_command.contains("stp-sidebar"));
}

#[test]
fn quit_binding_maps_raw_q_to_detach_client() {
    let tmux = Tmux::new("stp-cli-quit-binding-test");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");

    install_quit_binding(&tmux).expect("quit binding");

    let output = std::process::Command::new("tmux")
        .args([
            "-L",
            "stp-cli-quit-binding-test",
            "list-keys",
            "-T",
            "root",
            "q",
        ])
        .output()
        .expect("list keys");
    let stdout = String::from_utf8_lossy(&output.stdout);
    tmux.kill_server().expect("cleanup server");

    assert!(stdout.contains("bind-key -T root q detach-client"));
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
        status,
    }
}
