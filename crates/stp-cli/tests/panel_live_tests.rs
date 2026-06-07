#![allow(clippy::expect_used)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

#[test]
fn panel_ignores_live_registry_entries_when_tmux_session_is_missing() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-stale-pane");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-stale-pane-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-stale-pane-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-stale-panel";
    let terminal_id = "00000000-0000-0000-0000-000000000106";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    kill_tmux_server(&socket);

    Command::new("tmux")
        .args([
            "-L",
            &panel_socket,
            "new-session",
            "-d",
            "-s",
            panel_session,
            &format!(
                "STP_TMUX_SOCKET={} {} panel --registry {} --layout 2x2",
                shell_quote(&socket),
                shell_quote(&binary.display().to_string()),
                shell_quote(&registry.display().to_string()),
            ),
        ])
        .assert()
        .success();

    wait_for_pane_title(&socket, "empty:1");
    wait_for_titled_pane_capture(&socket, "empty:1", "slot 1: <empty>");
    assert!(registry_terminal_ids(&registry).is_empty());
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_attaches_and_controls_registered_terminal_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-live-pane");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-live-pane-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-live-pane-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-refresh-panel";
    let terminal_id = "00000000-0000-0000-0000-000000000105";
    let target_session = "stp-00000000-0000-0000-0000-000000000105";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
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
            &socket,
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--shell",
            "sh",
            "--detach",
        ])
        .assert()
        .success();
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "qa-send-focused",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_id,
            "--text",
            "echo panel-target-ready",
        ])
        .assert()
        .success();
    wait_for_tmux_capture(&socket, target_session, "panel-target-ready");

    Command::new("tmux")
        .args([
            "-L",
            &panel_socket,
            "new-session",
            "-d",
            "-s",
            panel_session,
            &format!(
                "STP_TMUX_SOCKET={} {} panel --registry {} --layout 3x3",
                shell_quote(&socket),
                shell_quote(&binary.display().to_string()),
                shell_quote(&registry.display().to_string()),
            ),
        ])
        .assert()
        .success();
    wait_for_pane_title(&socket, terminal_id);
    wait_for_titled_pane_capture(&socket, terminal_id, "panel-target-ready");

    let panel_target = wait_for_pane_title(&socket, terminal_id);
    Command::new("tmux")
        .args([
            "-L",
            &socket,
            "send-keys",
            "-t",
            &panel_target,
            "echo panel-input-routed",
            "Enter",
        ])
        .assert()
        .success();
    wait_for_tmux_capture(&socket, target_session, "panel-input-routed");
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
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

fn kill_tmux_server(socket: &str) {
    let _ = Command::new("tmux")
        .args(["-L", socket, "kill-server"])
        .ok();
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn wait_for_tmux_capture(socket: &str, session: &str, needle: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let target = format!("{session}:0.0");
    loop {
        let output = Command::new("tmux")
            .args(["-L", socket, "capture-pane", "-pt", &target, "-S", "-200"])
            .output()
            .expect("capture pane");
        let capture = String::from_utf8_lossy(&output.stdout);
        let unwrapped_capture = capture.replace(['\r', '\n'], "");
        if capture.contains(needle) || unwrapped_capture.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for tmux capture to contain {needle}; got {capture}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn wait_for_titled_pane_capture(socket: &str, title: &str, needle: &str) {
    let target = wait_for_pane_title(socket, title);
    wait_for_capture_target(socket, &target, needle);
}

fn wait_for_capture_target(socket: &str, target: &str, needle: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let output = Command::new("tmux")
            .args(["-L", socket, "capture-pane", "-pt", target, "-S", "-200"])
            .output()
            .expect("capture pane");
        let capture = String::from_utf8_lossy(&output.stdout);
        let unwrapped_capture = capture.replace(['\r', '\n'], "");
        if capture.contains(needle) || unwrapped_capture.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for tmux capture to contain {needle}; got {capture}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn wait_for_pane_title(socket: &str, expected_title: &str) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let output = Command::new("tmux")
            .args([
                "-L",
                socket,
                "list-panes",
                "-t",
                "stp-panel",
                "-F",
                concat!("#", "{pane_id}\t#", "{@stp-pane-key}"),
            ])
            .output()
            .expect("pane titles");
        let titles = String::from_utf8_lossy(&output.stdout);
        for line in titles.lines() {
            if let Some((pane_id, title)) = line.split_once('\t')
                && title == expected_title
            {
                return pane_id.to_owned();
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for pane title {expected_title}; got {titles}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn registry_terminal_ids(registry: &std::path::Path) -> Vec<String> {
    let raw = std::fs::read_to_string(registry).expect("registry json");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("registry value");
    parsed["terminals"]
        .as_array()
        .expect("terminals array")
        .iter()
        .map(|terminal| {
            terminal["terminal_id"]
                .as_str()
                .expect("terminal id")
                .to_owned()
        })
        .collect()
}
