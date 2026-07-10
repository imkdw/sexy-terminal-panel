#![allow(clippy::expect_used)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn invalid_workspace_fails_when_terminal_command_runs() {
    let mut cmd = Command::cargo_bin("stp").expect("stp binary");

    cmd.args([
        "terminal",
        "--workspace",
        "/path/that/does/not/exist",
        "--window-id",
        "00000000-0000-0000-0000-000000000001",
        "--terminal-id",
        "00000000-0000-0000-0000-000000000102",
        "--detach",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("workspace path does not exist"));
}

#[test]
fn panel_once_renders_registered_terminal_when_registry_exists() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-a");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminal",
            "--workspace",
            workspace.to_str().expect("utf8 workspace"),
            "--window-id",
            "00000000-0000-0000-0000-000000000001",
            "--terminal-id",
            "00000000-0000-0000-0000-000000000101",
            "--socket",
            "stp-cli-panel-test",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--detach",
        ])
        .assert()
        .success();

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "panel",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--layout",
            "3x3",
            "--once",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Layout: 3x3"))
        .stdout(predicate::str::contains("worktree-a"))
        .stdout(predicate::str::contains(
            "00000000-0000-0000-0000-000000000101",
        ));

    let _ = Command::new("tmux")
        .args(["-L", "stp-cli-panel-test", "kill-server"])
        .ok();
}

#[test]
fn panel_defaults_to_two_by_two_when_layout_is_omitted() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "panel",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--once",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Layout: 2x2"));
}

#[test]
fn send_focused_routes_to_registered_socket() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-routed");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-route-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000104";

    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .ok();
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
            "printf routed-through-registry",
        ])
        .assert()
        .success();

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "qa-capture",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_id,
            "--lines",
            "10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("routed-through-registry"));

    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .ok();
}

#[test]
fn open_cursor_prints_cursor_fallback_when_dry_run() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-cursor");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-open-cursor-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000105";

    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .ok();
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
            "--detach",
        ])
        .assert()
        .success();

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "open-cursor",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_id,
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("cursor --new-window"))
        .stdout(predicate::str::contains("worktree-cursor"));

    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .ok();
}

#[test]
fn terminal_attaches_when_launched_from_pty() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-pty");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-pty-test-{}", std::process::id());
    let window_id = "00000000-0000-0000-0000-000000000001";
    let terminal_id = "00000000-0000-0000-0000-000000000103";
    let session = "stp-00000000-0000-0000-0000-000000000103";
    let detach = format!(
        "for i in 1 2 3 4 5; do tmux -L {} detach-client -s {} >/dev/null 2>&1 && exit 0; sleep 0.2; done",
        shell_quote(&socket),
        shell_quote(session),
    );
    let command = format!(
        "({detach}) & STP_REGISTRY={} STP_TMUX_SOCKET={} {} terminal --workspace {} --window-id {} --terminal-id {}",
        shell_quote(&registry.display().to_string()),
        shell_quote(&socket),
        shell_quote(&binary.display().to_string()),
        shell_quote(&workspace.display().to_string()),
        shell_quote(window_id),
        shell_quote(terminal_id),
    );

    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .ok();
    Command::new("script")
        .args(["-q", "/dev/null", "sh", "-c", &command])
        .assert()
        .success();
    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .ok();
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
