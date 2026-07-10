mod panel_auto_connect_support;

use panel_auto_connect_support::{
    AutoConnectFixture, capture_pane, list_layout_panes, tmux_output, wait_for_layout_panes,
    wait_for_panel_pane_key, wait_for_success,
};

#[test]
fn panel_auto_connects_terminal_registered_after_panel_opened() {
    let fixture = AutoConnectFixture::new("serial");
    let panel_terminal_id = "00000000-0000-0000-0000-000000000109";
    let opened_terminal_id = "00000000-0000-0000-0000-000000000110";

    fixture.register(panel_terminal_id);
    fixture.launch_panel("2x2");
    let initial_panes = wait_for_layout_panes(&fixture.socket, 5);
    assert!(
        initial_panes
            .iter()
            .any(|pane| pane.key == panel_terminal_id),
        "panel must start with the terminal that opened it"
    );
    assert!(
        initial_panes.iter().any(|pane| pane.key == "empty:2"),
        "panel must have an empty slot for the next terminal"
    );

    fixture.register(opened_terminal_id);

    wait_for_panel_pane_key(&fixture.socket, opened_terminal_id);
    let panes = list_layout_panes(&fixture.socket);
    assert!(
        panes.iter().all(|pane| pane.key != "empty:2"),
        "new terminal should occupy the lowest empty slot: {panes:?}"
    );
    assert!(
        panes.iter().any(|pane| pane.key == "empty:3"),
        "higher empty slots should remain available: {panes:?}"
    );
    let sidebar = panes
        .iter()
        .find(|pane| pane.key == "stp-sidebar")
        .expect("sidebar pane");
    let sidebar_text = capture_pane(&fixture.socket, &sidebar.id);
    assert!(
        sidebar_text.contains("2 live sessions"),
        "sidebar should refresh after auto-connect: {sidebar_text}"
    );
    assert!(
        sidebar_text.contains(&opened_terminal_id[..8]),
        "sidebar should include the new terminal id: {sidebar_text}"
    );
}

#[test]
fn panel_auto_connects_concurrent_terminals_to_distinct_empty_slots() {
    let fixture = AutoConnectFixture::new("concurrent");
    let panel_terminal_id = "00000000-0000-0000-0000-000000000111";
    let opened_terminal_a = "00000000-0000-0000-0000-000000000112";
    let opened_terminal_b = "00000000-0000-0000-0000-000000000113";

    fixture.register(panel_terminal_id);
    fixture.launch_panel("2x2");
    wait_for_layout_panes(&fixture.socket, 5);

    let child_a = fixture.spawn_register(opened_terminal_a);
    let child_b = fixture.spawn_register(opened_terminal_b);
    wait_for_success(child_a);
    wait_for_success(child_b);

    wait_for_panel_pane_key(&fixture.socket, opened_terminal_a);
    wait_for_panel_pane_key(&fixture.socket, opened_terminal_b);
    let panes = list_layout_panes(&fixture.socket);
    assert!(
        panes.iter().all(|pane| pane.key != "empty:2"),
        "first concurrent terminal should claim slot 2: {panes:?}"
    );
    assert!(
        panes.iter().all(|pane| pane.key != "empty:3"),
        "second concurrent terminal should claim slot 3: {panes:?}"
    );
    assert!(
        panes.iter().any(|pane| pane.key == "empty:4"),
        "one empty slot should remain: {panes:?}"
    );
}

#[test]
fn terminal_registration_succeeds_when_panel_is_not_open() {
    let fixture = AutoConnectFixture::new("no-panel");
    let terminal_id = "00000000-0000-0000-0000-000000000114";

    fixture.register(terminal_id);

    let sessions = tmux_output(&fixture.socket, &["list-sessions", "-F", "#{session_name}"]);
    assert!(sessions.contains(&format!("stp-{terminal_id}")));
    assert!(!sessions.contains("stp-panel"));
}
