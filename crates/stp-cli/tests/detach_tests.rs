#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn detach_hides_terminal_from_panel_without_killing_tmux_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-detach");
    fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-detach-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000904";
    let tmux_session = format!("stp-{terminal_id}");

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "detach",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("detached {terminal_id}")));

    assert_tmux_session_exists(&socket, &tmux_session);
    assert_eq!(registry_status(&registry, terminal_id), "detached");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "panel",
            "--once",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--layout",
            "2x2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No live STP sessions"))
        .stdout(predicate::str::contains(terminal_id).not());

    kill_tmux_server(&socket);
}

fn register_detached_terminal(registry: &Path, workspace: &Path, socket: &str, terminal_id: &str) {
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

fn assert_tmux_session_exists(socket: &str, session: &str) {
    Command::new("tmux")
        .args(["-L", socket, "has-session", "-t", session])
        .assert()
        .success();
}

fn registry_status(registry: &Path, terminal_id: &str) -> String {
    let raw = fs::read_to_string(registry).expect("registry json");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("registry value");
    parsed["terminals"]
        .as_array()
        .expect("terminals array")
        .iter()
        .find(|terminal| terminal["terminal_id"] == terminal_id)
        .expect("registered terminal")["status"]
        .as_str()
        .expect("status string")
        .to_owned()
}
