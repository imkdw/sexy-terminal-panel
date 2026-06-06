#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn terminate_kills_managed_tmux_session_when_confirmed() {
    let temp = TempDir::new().expect("temp dir");
    let workspace_a = temp.path().join("worktree-terminate-a");
    let workspace_b = temp.path().join("worktree-terminate-b");
    fs::create_dir(&workspace_a).expect("workspace a");
    fs::create_dir(&workspace_b).expect("workspace b");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-terminate-test-{}", std::process::id());
    let terminal_a = "00000000-0000-0000-0000-000000000501";
    let terminal_b = "00000000-0000-0000-0000-000000000502";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace_a, &socket, terminal_a);
    register_detached_terminal(&registry, &workspace_b, &socket, terminal_b);

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminate",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_b,
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("terminated {terminal_b}")));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_a}"));
    assert_tmux_session_missing(&socket, &format!("stp-{terminal_b}"));
    assert_eq!(registry_status(&registry, terminal_b), "exited");
    kill_tmux_server(&socket);
}

#[test]
fn terminate_marks_exited_when_session_already_missing() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-terminate-missing");
    fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-terminate-missing-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000503";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    kill_tmux_server(&socket);

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminate",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_id,
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "already exited {terminal_id}"
        )));

    assert_eq!(registry_status(&registry, terminal_id), "exited");
}

#[test]
fn terminate_refuses_without_yes() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminate",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            "00000000-0000-0000-0000-000000000504",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to terminate without --yes",
        ));
}

#[test]
fn terminate_fails_when_terminal_id_is_unknown() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");
    let terminal_id = "00000000-0000-0000-0000-000000000505";

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminate",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            terminal_id,
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "terminal not found: {terminal_id}"
        )));
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

fn assert_tmux_session_missing(socket: &str, session: &str) {
    Command::new("tmux")
        .args(["-L", socket, "has-session", "-t", session])
        .assert()
        .failure();
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
