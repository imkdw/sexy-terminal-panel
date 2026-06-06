#![allow(clippy::expect_used)]

use std::process::Command;

use stp_tmux::adapter::{BindingCommand, Tmux};

#[test]
fn send_keys_to_pane_when_isolated_server_running() {
    let tmux = Tmux::new("stp-tmux-adapter-send");
    tmux.kill_server().ok();
    tmux.new_session("stp-send", "sh").expect("new session");
    tmux.send_keys("stp-send", "printf hello-from-stp", true)
        .expect("send keys");

    let capture = tmux.capture_pane("stp-send", 50).expect("capture pane");
    tmux.kill_server().expect("cleanup server");

    assert!(capture.contains("hello-from-stp"));
}

#[test]
fn missing_target_returns_error_when_session_absent() {
    let tmux = Tmux::new("stp-tmux-adapter-missing");
    tmux.kill_server().ok();

    let err = tmux
        .capture_pane("missing-session", 10)
        .expect_err("missing target");

    assert!(err.to_string().contains("tmux command failed"));
    assert!(err.to_string().contains("missing-session"));
}

#[test]
fn split_window_can_build_nine_pane_layout() {
    let tmux = Tmux::new("stp-tmux-adapter-split");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");

    for _slot in 2..=9 {
        tmux.split_window("stp-panel", "sh").expect("split pane");
        tmux.select_tiled_layout("stp-panel").expect("tiled layout");
    }

    let output = Command::new("tmux")
        .args([
            "-L",
            "stp-tmux-adapter-split",
            "list-panes",
            "-t",
            "stp-panel",
        ])
        .output()
        .expect("list panes");
    let pane_count = String::from_utf8_lossy(&output.stdout).lines().count();
    tmux.kill_server().expect("cleanup server");

    assert_eq!(pane_count, 9);
}

#[test]
fn bind_key_registers_prefix_binding() {
    let tmux = Tmux::new("stp-tmux-adapter-bind-key");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    tmux.bind_key("K", "display-message 'STP terminate'")
        .expect("bind key");

    let output = Command::new("tmux")
        .args([
            "-L",
            "stp-tmux-adapter-bind-key",
            "list-keys",
            "-T",
            "prefix",
            "K",
        ])
        .output()
        .expect("list keys");
    let stdout = String::from_utf8_lossy(&output.stdout);
    tmux.kill_server().expect("cleanup server");

    assert!(stdout.contains("display-message"));
    assert!(stdout.contains("STP terminate"));
}

#[test]
fn bind_key_in_table_registers_root_mouse_binding() {
    let tmux = Tmux::new("stp-tmux-adapter-root-bind-key");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    tmux.bind_key_in_table(
        "root",
        "MouseDown1Pane",
        &BindingCommand::if_shell_format(
            "#{==:#{pane_title},stp-sidebar}",
            "display-message sidebar",
        ),
    )
    .expect("bind key");

    let output = Command::new("tmux")
        .args([
            "-L",
            "stp-tmux-adapter-root-bind-key",
            "list-keys",
            "-T",
            "root",
            "MouseDown1Pane",
        ])
        .output()
        .expect("list keys");
    let stdout = String::from_utf8_lossy(&output.stdout);
    tmux.kill_server().expect("cleanup server");

    assert!(stdout.contains("MouseDown1Pane"));
    assert!(stdout.contains("stp-sidebar"));
}

#[test]
fn set_pane_title_updates_selected_pane_title() {
    let tmux = Tmux::new("stp-tmux-adapter-pane-title");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    tmux.set_pane_title("stp-panel:0.0", "00000000-0000-0000-0000-000000000601")
        .expect("pane title");

    let output = Command::new("tmux")
        .args([
            "-L",
            "stp-tmux-adapter-pane-title",
            "list-panes",
            "-t",
            "stp-panel",
            "-F",
            "#{pane_title}",
        ])
        .output()
        .expect("list panes");
    let stdout = String::from_utf8_lossy(&output.stdout);
    tmux.kill_server().expect("cleanup server");

    assert!(stdout.contains("00000000-0000-0000-0000-000000000601"));
}

#[test]
fn list_pane_ids_returns_stable_targets() {
    let tmux = Tmux::new("stp-tmux-adapter-pane-ids");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    tmux.split_window("stp-panel", "sh").expect("split pane");

    let pane_ids = tmux.list_pane_ids("stp-panel").expect("pane ids");
    tmux.kill_server().expect("cleanup server");

    assert_eq!(pane_ids.len(), 2);
    assert!(pane_ids.iter().all(|pane_id| pane_id.starts_with('%')));
}

#[test]
fn list_panes_with_titles_returns_ids_and_titles() {
    let tmux = Tmux::new("stp-tmux-adapter-pane-info");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "sh").expect("new session");
    let pane_id = tmux.list_pane_ids("stp-panel").expect("pane ids")[0].clone();
    tmux.set_pane_title(&pane_id, "stp-sidebar")
        .expect("pane title");
    tmux.set_pane_option(&pane_id, "@stp-pane-key", "stp-sidebar")
        .expect("pane key");

    let panes = tmux.list_panes_with_titles("stp-panel").expect("pane info");
    tmux.kill_server().expect("cleanup server");

    assert_eq!(
        panes,
        vec![stp_tmux::adapter::PaneInfo {
            pane_id,
            title: "stp-sidebar".to_owned(),
            pane_key: "stp-sidebar".to_owned(),
        }]
    );
}

#[test]
fn respawn_pane_replaces_command_output() {
    let tmux = Tmux::new("stp-tmux-adapter-respawn");
    tmux.kill_server().ok();
    tmux.new_session("stp-panel", "printf before; exec sh")
        .expect("new session");
    let pane_id = tmux.list_pane_ids("stp-panel").expect("pane ids")[0].clone();

    tmux.respawn_pane(&pane_id, "printf after; exec sh")
        .expect("respawn");
    let capture = wait_for_capture(&tmux, &pane_id, "after");
    tmux.kill_server().expect("cleanup server");

    assert!(capture.contains("after"));
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
