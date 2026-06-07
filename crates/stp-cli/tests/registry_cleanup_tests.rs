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
fn panel_once_removes_zombie_entry_before_rendering_sidebar() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-zombie");
    fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-panel-zombie-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000902";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    kill_tmux_server(&socket);

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

    assert!(registry_terminal_ids(&registry).is_empty());
}

#[test]
fn panel_once_keeps_existing_stale_entries_for_explicit_cleanup() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-stale");
    fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = format!("stp-cli-panel-stale-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000903";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    mark_terminal_status(&registry, "stale");
    kill_tmux_server(&socket);

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

    assert_eq!(
        registry_terminal_ids(&registry),
        vec![terminal_id.to_owned()]
    );
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

fn mark_terminal_status(registry: &Path, status: &str) {
    let raw = fs::read_to_string(registry).expect("registry json");
    let mut parsed: serde_json::Value = serde_json::from_str(&raw).expect("registry value");
    let terminals = parsed["terminals"].as_array_mut().expect("terminals array");
    for terminal in terminals {
        terminal["status"] = serde_json::Value::String(status.to_owned());
    }
    fs::write(
        registry,
        serde_json::to_string_pretty(&parsed).expect("registry json"),
    )
    .expect("write registry");
}
