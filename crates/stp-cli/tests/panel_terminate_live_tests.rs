#![allow(clippy::expect_used)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

mod panel_terminate_support;

use panel_terminate_support::{
    assert_tmux_session_exists, kill_tmux_server, launch_panel, register_detached_terminal,
    send_prefix_k, wait_for_missing_tmux_session, wait_for_pane_title,
};

#[test]
fn panel_prefix_k_terminates_only_active_managed_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace_a = temp.path().join("worktree-panel-k-a");
    let workspace_b = temp.path().join("worktree-panel-k-b");
    std::fs::create_dir(&workspace_a).expect("workspace a");
    std::fs::create_dir(&workspace_b).expect("workspace b");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-wrapper";
    let terminal_a = "00000000-0000-0000-0000-000000000701";
    let terminal_b = "00000000-0000-0000-0000-000000000702";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace_a, &socket, terminal_a);
    register_detached_terminal(&registry, &workspace_b, &socket, terminal_b);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    wait_for_pane_title(&socket, terminal_a);
    let terminal_b_pane = wait_for_pane_title(&socket, terminal_b);

    Command::new("tmux")
        .args(["-L", &socket, "select-pane", "-t", &terminal_b_pane])
        .assert()
        .success();
    send_prefix_k(&socket, "stp-panel");
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
        .success();
    wait_for_missing_tmux_session(&socket, &format!("stp-{terminal_b}"));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_a}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_prefix_k_decline_preserves_session() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-k-decline");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-decline-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-decline-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-decline-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000703";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    wait_for_pane_title(&socket, terminal_id);

    send_prefix_k(&socket, "stp-panel");
    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "n"])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_id}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_prefix_k_on_empty_pane_is_noop() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-k-empty");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-empty-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-empty-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-empty-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000704";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    let empty_pane = wait_for_pane_title(&socket, "empty:2");

    Command::new("tmux")
        .args(["-L", &socket, "select-pane", "-t", &empty_pane])
        .assert()
        .success();
    send_prefix_k(&socket, "stp-panel");
    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "y"])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_id}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_prefix_k_on_sidebar_is_noop() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-panel-k-sidebar");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-sidebar-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-sidebar-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-sidebar-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000706";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    let sidebar_pane = wait_for_pane_title(&socket, "stp-sidebar");

    Command::new("tmux")
        .args(["-L", &socket, "select-pane", "-t", &sidebar_pane])
        .assert()
        .success();
    send_prefix_k(&socket, "stp-panel");
    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "y"])
        .assert()
        .success();
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_id}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}

#[test]
fn panel_prefix_k_after_sidebar_click_targets_selected_right_pane() {
    let temp = TempDir::new().expect("temp dir");
    let workspace_a = temp.path().join("worktree-panel-k-click-a");
    let workspace_b = temp.path().join("worktree-panel-k-click-b");
    std::fs::create_dir(&workspace_a).expect("workspace a");
    std::fs::create_dir(&workspace_b).expect("workspace b");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-panel-k-click-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-panel-k-click-outer-test-{}", std::process::id());
    let panel_session = "stp-cli-panel-k-click-wrapper";
    let terminal_a = "00000000-0000-0000-0000-000000000707";
    let terminal_b = "00000000-0000-0000-0000-000000000708";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace_a, &socket, terminal_a);
    register_detached_terminal(&registry, &workspace_b, &socket, terminal_b);
    launch_panel(&panel_socket, panel_session, &socket, &binary, &registry);
    wait_for_pane_title(&socket, terminal_a);
    wait_for_pane_title(&socket, terminal_b);
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "panel-select",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--socket",
            &socket,
            "--mouse-line",
            "4",
        ])
        .assert()
        .success();
    let active_terminal = panel_terminate_support::active_pane_key(&socket);
    assert_eq!(active_terminal, terminal_b);
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminate",
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--terminal-id",
            &active_terminal,
            "--yes",
        ])
        .assert()
        .success();
    wait_for_missing_tmux_session(&socket, &format!("stp-{terminal_b}"));

    assert_tmux_session_exists(&socket, &format!("stp-{terminal_a}"));
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}
