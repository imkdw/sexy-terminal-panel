#![allow(clippy::expect_used)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

mod panel_layout_support;

use panel_layout_support::{
    assert_content_panes, assert_sidebar_does_not_wrap, kill_tmux_server, launch_wrapped_panel,
    register_detached_terminal, wait_for_layout_panes, wait_for_panel_client_count,
};

#[test]
fn panel_creates_left_sidebar_and_right_grid() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-sidebar-layout");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-sidebar-layout-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000107";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);

    for (layout, capacity) in [("2x2", 4usize), ("3x3", 9usize)] {
        let panel_socket = format!(
            "stp-cli-sidebar-layout-outer-{layout}-{}",
            std::process::id()
        );
        let panel_session = format!("stp-cli-sidebar-layout-{layout}");
        kill_tmux_server(&panel_socket);
        launch_wrapped_panel(
            &panel_socket,
            &panel_session,
            &socket,
            &binary,
            &registry,
            layout,
        );

        let panes = wait_for_layout_panes(&socket, capacity.saturating_add(1));
        let sidebar = panes
            .iter()
            .find(|pane| pane.key == "stp-sidebar")
            .expect("sidebar pane");
        assert_eq!(sidebar.left, 0, "{layout} sidebar must stay leftmost");
        assert_eq!(sidebar.width, 30, "{layout} sidebar width");
        assert_content_panes(layout, terminal_id, capacity, &panes, sidebar);
        assert_sidebar_does_not_wrap(layout, &socket, &sidebar.id);
        kill_tmux_server(&panel_socket);
    }

    kill_tmux_server(&socket);
}

#[test]
fn panel_raw_q_detaches_panel_client_instead_of_reaching_terminal() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-quit-panel");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-quit-panel-test-{}", std::process::id());
    let panel_socket = format!("stp-cli-quit-panel-outer-{}", std::process::id());
    let panel_session = "stp-cli-quit-panel-wrapper";
    let terminal_id = "00000000-0000-0000-0000-000000000108";

    kill_tmux_server(&socket);
    kill_tmux_server(&panel_socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);
    launch_wrapped_panel(
        &panel_socket,
        panel_session,
        &socket,
        &binary,
        &registry,
        "2x2",
    );
    wait_for_layout_panes(&socket, 5);

    Command::new("tmux")
        .args(["-L", &panel_socket, "send-keys", "-t", panel_session, "q"])
        .assert()
        .success();

    wait_for_panel_client_count(&socket, 0);
    kill_tmux_server(&panel_socket);
    kill_tmux_server(&socket);
}
