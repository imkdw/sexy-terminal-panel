#![allow(clippy::expect_used)]

use std::process::Command;

use stp_tmux::adapter::{Tmux, TmuxError};

#[test]
fn public_adapter_surface_hides_raw_command_args() {
    let adapter_source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/adapter.rs"))
            .expect("adapter source");

    assert!(!adapter_source.contains("pub fn bind_key_args"));
    assert!(!adapter_source.contains("pub fn bind_key_in_table_args"));
    assert!(!adapter_source.contains("args: &[&str]"));
}

#[test]
fn production_tmux_adapter_avoids_unwrap_and_expect() {
    for source_path in [
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/adapter.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/error.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/runner.rs"),
    ] {
        let source = std::fs::read_to_string(source_path).expect("production source");

        assert!(
            !source.contains("unwrap"),
            "{source_path} contains production unwrap"
        );
        assert!(
            !source.contains("expect"),
            "{source_path} contains production expect"
        );
    }
}

#[test]
fn missing_target_error_includes_socket_command_status_stdout_stderr() {
    let tmux = Tmux::new("stp-tmux-adapter-missing-context");
    tmux.kill_server().ok();

    let err = tmux
        .capture_pane("missing-session", 10)
        .expect_err("missing target");

    match err {
        TmuxError::CommandFailed {
            socket,
            command,
            status,
            stdout,
            stderr,
        } => {
            assert_eq!(socket, "stp-tmux-adapter-missing-context");
            assert!(command.contains("capture-pane"));
            assert!(command.contains("missing-session"));
            assert_ne!(status, 0);
            assert!(stdout.is_empty() || stderr.is_empty() || !stdout.eq(&stderr));
        }
        TmuxError::Spawn { .. } => panic!("expected command failure"),
    }
}

#[test]
fn split_window_with_id_returns_new_stable_pane_id() {
    let tmux = Tmux::new("stp-tmux-adapter-split-with-id");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    let existing = tmux.list_pane_ids("stp-panel").expect("pane ids");

    let new_pane_id = tmux
        .split_window_with_id("stp-panel", "printf split-id; exec sh")
        .expect("split with id");

    let updated = tmux.list_pane_ids("stp-panel").expect("pane ids");
    tmux.kill_server().expect("cleanup server");

    assert!(new_pane_id.starts_with('%'));
    assert!(!existing.contains(&new_pane_id));
    assert!(updated.contains(&new_pane_id));
}

#[test]
fn split_window_right_returns_new_pane_id() {
    let tmux = Tmux::new("stp-tmux-adapter-split-right");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    let existing = tmux.list_pane_ids("stp-panel").expect("pane ids");

    let new_pane_id = tmux
        .split_window_right_with_id("stp-panel", "printf right; exec sh")
        .expect("split right");

    let updated = tmux.list_pane_ids("stp-panel").expect("pane ids");
    tmux.kill_server().expect("cleanup server");

    assert!(new_pane_id.starts_with('%'));
    assert!(!existing.contains(&new_pane_id));
    assert!(updated.contains(&new_pane_id));
}

#[test]
fn resize_pane_width_updates_pane_width() {
    let tmux = Tmux::new("stp-tmux-adapter-resize-width");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    let existing = tmux.list_pane_ids("stp-panel").expect("pane ids");
    tmux.split_window_left("stp-panel", 20, "sh")
        .expect("split pane");
    let pane_id = tmux
        .list_pane_ids("stp-panel")
        .expect("pane ids")
        .into_iter()
        .find(|pane_id| !existing.contains(pane_id))
        .expect("new pane id");

    tmux.resize_pane_width(&pane_id, 25).expect("resize pane");

    let width = pane_width("stp-tmux-adapter-resize-width", &pane_id);
    tmux.kill_server().expect("cleanup server");

    assert_eq!(width, 25);
}

#[test]
fn select_and_respawn_pane_update_active_pane_and_command() {
    let tmux = Tmux::new("stp-tmux-adapter-select-respawn");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "printf before; exec sh")
        .expect("new session");
    let first_pane_id = tmux.list_pane_ids("stp-panel").expect("pane ids")[0].clone();
    let second_pane_id = tmux
        .split_window_with_id("stp-panel", "printf second; exec sh")
        .expect("split with id");

    tmux.respawn_pane(&second_pane_id, "printf after; exec sh")
        .expect("respawn");
    tmux.select_pane(&second_pane_id).expect("select pane");
    let capture = wait_for_capture(&tmux, &second_pane_id, "after");
    let active = active_pane_id("stp-tmux-adapter-select-respawn", "stp-panel");
    tmux.kill_server().expect("cleanup server");

    assert_ne!(first_pane_id, second_pane_id);
    assert_eq!(active, second_pane_id);
    assert!(capture.contains("after"));
}

fn pane_width(socket: &str, pane_id: &str) -> usize {
    let output = Command::new("tmux")
        .args([
            "-L",
            socket,
            "display-message",
            "-p",
            "-t",
            pane_id,
            "#{pane_width}",
        ])
        .output()
        .expect("pane width");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .expect("numeric width")
}

fn active_pane_id(socket: &str, session: &str) -> String {
    let output = Command::new("tmux")
        .args([
            "-L",
            socket,
            "list-panes",
            "-t",
            session,
            "-F",
            "#{pane_id}\t#{pane_active}",
        ])
        .output()
        .expect("active pane");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            let (pane_id, active) = line.split_once('\t')?;
            (active == "1").then(|| pane_id.to_owned())
        })
        .expect("active pane id")
}

fn wait_for_capture(tmux: &Tmux, target: &str, needle: &str) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let capture = tmux.capture_pane(target, 20).expect("capture");
        if capture.contains(needle) {
            return capture;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for capture to contain {needle}; got {capture}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
