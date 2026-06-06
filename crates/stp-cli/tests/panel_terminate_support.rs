#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(dead_code)]

use std::process::Command as ProcessCommand;

use assert_cmd::Command;

pub fn kill_tmux_server(socket: &str) {
    let _ = Command::new("tmux")
        .args(["-L", socket, "kill-server"])
        .ok();
}

pub fn register_detached_terminal(
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

pub fn launch_panel(
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

pub fn send_prefix_k(socket: &str, session: &str) {
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

pub fn assert_tmux_session_exists(socket: &str, session: &str) {
    Command::new("tmux")
        .args(["-L", socket, "has-session", "-t", session])
        .assert()
        .success();
}

pub fn wait_for_missing_tmux_session(socket: &str, session: &str) {
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

pub fn wait_for_pane_title(socket: &str, expected_title: &str) -> String {
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
                concat!("#", "{pane_id}\t#", "{@stp-pane-key}"),
            ])
            .output()
            .expect("pane titles");
        let titles = String::from_utf8_lossy(&output.stdout);
        for line in titles.lines() {
            if let Some((pane_id, pane_key)) = line.split_once('\t')
                && pane_key == expected_title
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

pub fn active_pane_key(socket: &str) -> String {
    let output = ProcessCommand::new("tmux")
        .args([
            "-L",
            socket,
            "list-panes",
            "-t",
            "stp-panel",
            "-F",
            "#{pane_active}\t#{@stp-pane-key}",
        ])
        .output()
        .expect("active pane");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            let (active, key) = line.split_once('\t')?;
            (active == "1").then(|| key.to_owned())
        })
        .expect("active pane key")
}

pub fn wait_for_active_pane_key(socket: &str, expected_key: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let active = active_pane_key(socket);
        if active == expected_key {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for active pane {expected_key}; got {active}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub fn pane_keys(socket: &str) -> Vec<String> {
    let output = ProcessCommand::new("tmux")
        .args([
            "-L",
            socket,
            "list-panes",
            "-t",
            "stp-panel",
            "-F",
            "#{@stp-pane-key}",
        ])
        .output()
        .expect("pane keys");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

pub fn set_pane_key(socket: &str, pane_id: &str, key: &str) {
    Command::new("tmux")
        .args(["-L", socket, "select-pane", "-t", pane_id, "-T", key])
        .assert()
        .success();
    Command::new("tmux")
        .args([
            "-L",
            socket,
            "set-option",
            "-p",
            "-t",
            pane_id,
            "@stp-pane-key",
            key,
        ])
        .assert()
        .success();
}

pub fn tmux_messages(socket: &str) -> String {
    let output = ProcessCommand::new("tmux")
        .args(["-L", socket, "show-messages"])
        .output()
        .expect("tmux messages");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
