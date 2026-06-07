#![allow(clippy::expect_used)]

use assert_cmd::Command;
use predicates::prelude::*;
use stp_pty::pid_path_for_socket;
use tempfile::TempDir;

#[test]
fn broker_ensure_starts_detached_server_and_status_reports_ready() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");
    let socket = temp.path().join("stp.sock");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "ensure",
            "--registry",
            registry.to_str().expect("registry"),
            "--socket",
            socket.to_str().expect("socket"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("broker ready"));

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "status",
            "--socket",
            socket.to_str().expect("socket"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("broker ready"));

    stop_broker(&socket);
}

#[test]
fn broker_stop_removes_socket_and_pid_files() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");
    let socket = temp.path().join("stp.sock");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "ensure",
            "--registry",
            registry.to_str().expect("registry"),
            "--socket",
            socket.to_str().expect("socket"),
        ])
        .assert()
        .success();

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "stop",
            "--socket",
            socket.to_str().expect("socket"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("stopped broker"));

    assert!(!socket.exists());
    assert!(!pid_path_for_socket(&socket).exists());
}

#[test]
fn broker_uses_default_state_socket_when_socket_arg_is_omitted() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");
    let socket = temp.path().join("sexy-terminal-panel/broker.sock");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .env("XDG_STATE_HOME", temp.path())
        .env_remove("STP_BROKER_SOCKET")
        .args([
            "broker",
            "ensure",
            "--registry",
            registry.to_str().expect("registry"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("broker ready"));

    Command::cargo_bin("stp")
        .expect("stp binary")
        .env("XDG_STATE_HOME", temp.path())
        .env_remove("STP_BROKER_SOCKET")
        .args(["broker", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("broker ready"));

    Command::cargo_bin("stp")
        .expect("stp binary")
        .env("XDG_STATE_HOME", temp.path())
        .env_remove("STP_BROKER_SOCKET")
        .args(["broker", "stop"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stopped broker"));

    assert!(!socket.exists());
    assert!(!pid_path_for_socket(&socket).exists());
}

#[test]
fn broker_cli_spawns_inputs_captures_and_terminates_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let socket = temp.path().join("stp.sock");
    let terminal_id = "00000000-0000-0000-0000-000000000801";

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "ensure",
            "--registry",
            registry.to_str().expect("registry"),
            "--socket",
            socket.to_str().expect("socket"),
        ])
        .assert()
        .success();
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "spawn",
            "--registry",
            registry.to_str().expect("registry"),
            "--socket",
            socket.to_str().expect("socket"),
            "--workspace",
            workspace.to_str().expect("workspace"),
            "--window-id",
            "00000000-0000-0000-0000-000000000701",
            "--terminal-id",
            terminal_id,
            "--shell",
            "sh",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("spawned"));
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "input",
            "--socket",
            socket.to_str().expect("socket"),
            "--terminal-id",
            terminal_id,
            "--text",
            "printf broker-ok\\n\r",
        ])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(150));

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "capture",
            "--socket",
            socket.to_str().expect("socket"),
            "--terminal-id",
            terminal_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("broker-ok"));

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "terminate",
            "--socket",
            socket.to_str().expect("socket"),
            "--terminal-id",
            terminal_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("terminated"));
    stop_broker(&socket);
}

#[test]
fn doctor_checks_broker_not_tmux() {
    let temp = TempDir::new().expect("temp dir");
    let registry = temp.path().join("registry.json");
    let socket = temp.path().join("stp.sock");

    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "doctor",
            "--registry",
            registry.to_str().expect("registry"),
            "--broker-socket",
            socket.to_str().expect("socket"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor ok"))
        .stdout(predicate::str::contains("broker socket"))
        .stdout(predicate::str::contains("tmux").not());
}

fn stop_broker(socket: &std::path::Path) {
    let _ = Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "broker",
            "stop",
            "--socket",
            socket.to_str().expect("socket"),
        ])
        .ok();
}
