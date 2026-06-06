#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn cleanup_zombies_removes_live_registry_entry_when_tmux_session_is_missing() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-cleanup-zombie");
    fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-cleanup-zombie-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000901";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    kill_tmux_server(&socket);

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "registry",
            "cleanup-zombies",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed zombie entries: 1"));

    assert!(registry_terminal_ids(&registry).is_empty());
}

#[test]
fn cleanup_zombies_refuses_without_yes() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "registry",
            "cleanup-zombies",
            "--registry",
            registry.to_str().expect("utf8 registry"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to cleanup zombie entries without --yes",
        ));
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

fn registry_terminal_ids(registry: &Path) -> Vec<String> {
    let raw = fs::read_to_string(registry).expect("registry json");
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
