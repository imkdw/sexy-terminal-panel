#![allow(clippy::expect_used)]

use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

#[test]
fn panel_prefix_k_terminates_only_active_managed_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace_a = temp.path().join("worktree-panel-k-a");
    let workspace_b = temp.path().join("worktree-panel-k-b");
    std::fs::create_dir(&workspace_a).expect("workspace a");
    std::fs::create_dir(&workspace_b).expect("workspace b");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-wrapper";
    let terminal_a = "00000000-0000-0000-0000-000000000701";
    let terminal_b = "00000000-0000-0000-0000-000000000702";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace_a, &socket, terminal_a);
    register_detached_terminal(&registry, &workspace_b, &socket, terminal_b);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    wait_for_pane_title(&socket, 0, terminal_a);
    wait_for_pane_title(&socket, 1, terminal_b);

    Command::new("tmux")
        .args(["-L", &socket, "select-pane", "-t", "stp-panel:0.1"])
        .assert()
        .success();
    send_prefix_k(&panel_socket, panel_session);
    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "y"])
        .assert()
        .success();
    wait_for_missing_tmux_session(&socket, &format!("stp-{terminal_b}"));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_a}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_prefix_k_decline_preserves_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-k-decline");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-decline-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-decline-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-decline-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000703";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    wait_for_pane_title(&socket, 0, terminal_id);

    send_prefix_k(&panel_socket, panel_session);
    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "n"])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_id}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_prefix_k_on_empty_pane_is_noop() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-k-empty");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-empty-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-empty-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-empty-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000704";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    wait_for_pane_title(&socket, 1, "empty:2");

    Command::new("tmux")
        .args(["-L", &socket, "select-pane", "-t", "stp-panel:0.1"])
        .assert()
        .success();
    send_prefix_k(&panel_socket, panel_session);
    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "y"])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_id}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

fn kill_tmux_server(socket: &str) {
    let _ = Command::new("tmux")
        .args(["-L", socket, "kill-server"])
        .ok();
}

fn register_detached_terminal(
    registry: &std::path::Path,
    workspace: &std::path::Path,
    socket: &str,
    terminal_id: &str,
) {
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminal",
            "--workspace",
            workspace.to_str().expect("utf8 workspace"),
            "--window-id",
            "00000000-0000-0000-0000-000000000001",
            "--terminal-id",
            terminal_id,
            "--socket",
            socket,
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--shell",
            "sh",
            "--detach",
        ])
        .assert()
        .success();
}

fn launch_panel(
    panel_socket: &str,
    panel_session: &str,
    managed_socket: &str,
    binary: &std::path::Path,
    registry: &std::path::Path,
) {
    Command::new("tmux")
        .args([
            "-L",
            panel_socket,
            "new-session",
            "-d",
            "-s",
            panel_session,
            &format!(
                "STP_TMUX_SOCKET={} {} panel --registry {} --layout 3x3",
                shell_quote(managed_socket),
                shell_quote(&binary.display().to_string()),
                shell_quote(&registry.display().to_string()),
            ),
        ])
        .assert()
        .success();
}

fn send_prefix_k(socket: &str, session: &str) {
    Command::new("tmux")
        .args(["-L", socket, "send-prefix", "-t", session])
        .assert()
        .success();
    Command::new("tmux")
        .args(["-L", socket, "send-keys", "-t", session, "K"])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(150));
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn assert_tmux_session_exists(socket: &str, session: &str) {
    Command::new("tmux")
        .args(["-L", socket, "has-session", "-t", session])
        .assert()
        .success();
}

fn wait_for_missing_tmux_session(socket: &str, session: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status = ProcessCommand::new("tmux")
            .args(["-L", socket, "has-session", "-t", session])
            .status()
            .expect("has session");
        if !status.success() {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for tmux session {session} to terminate"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn wait_for_pane_title(socket: &str, pane_index: usize, expected_title: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let target = format!("stp-panel:0.{pane_index}");
    loop {
        let output = ProcessCommand::new("tmux")
            .args([
                "-L",
                socket,
                "display-message",
                "-p",
                "-t",
                &target,
                "#{pane_title}",
            ])
            .output()
            .expect("pane title");
        let title = String::from_utf8_lossy(&output.stdout);
        if title.trim() == expected_title {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for pane {target} title {expected_title}; got {title}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
