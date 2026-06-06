#![allow(clippy::expect_used)]

use std::process::Command;

use stp_tmux::adapter::Tmux;

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
