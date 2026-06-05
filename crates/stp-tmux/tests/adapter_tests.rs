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
