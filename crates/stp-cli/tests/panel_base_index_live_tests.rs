#![allow(clippy::expect_used)]

use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

#[test]
fn panel_sets_titles_when_tmux_pane_base_index_is_nonzero() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-base-index");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-base-index-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-base-index-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-base-index-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000705";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    set_pane_base_index(&socket, "1");
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);

    wait_for_any_pane_title(&socket, terminal_id);
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

fn kill_tmux_server(socket: &str) {
    let _ = Command::new("tmux")
        .args(["-L", socket, "kill-server"])
        .ok();
}

fn set_pane_base_index(socket: &str, value: &str) {
    Command::new("tmux")
        .args([
            "-L",
            socket,
            "new-session",
            "-d",
            "-s",
            "base-index-anchor",
            "sh",
        ])
        .assert()
        .success();
    Command::new("tmux")
        .args(["-L", socket, "set-option", "-g", "pane-base-index", value])
        .assert()
        .success();
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn wait_for_any_pane_title(socket: &str, expected_title: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let output = ProcessCommand::new("tmux")
            .args([
                "-L",
                socket,
                "list-panes",
                "-t",
                "stp-panel",
                "-F",
                "#{pane_index}:#{@stp-pane-key}",
            ])
            .output()
            .expect("pane titles");
        let titles = String::from_utf8_lossy(&output.stdout);
        if titles.lines().any(|line| line.ends_with(expected_title)) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for pane title {expected_title}; got {titles}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
