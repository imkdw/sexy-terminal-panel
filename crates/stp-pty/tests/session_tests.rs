#![allow(clippy::expect_used)]

use stp_core::ids::TerminalId;
use stp_core::protocol::{ClientRequest, ServerEvent};
use stp_core::registry::{RegistryStore, TerminalBackend, TerminalStatus};

mod session_support;

use session_support::{
    BrokerFixture, assert_output_contains, assert_process_exited, assert_session_removed_from_list,
};

#[test]
fn broker_spawn_creates_live_registry_terminal_and_shell_pid() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000701");
    let spawned = fixture.spawn();

    assert!(spawned.process_id.is_some());
    let registry = RegistryStore::new(fixture.config.registry_path.clone())
        .load()
        .expect("registry");
    let terminal = registry
        .terminal(&fixture.terminal_id)
        .expect("registered terminal");
    assert_eq!(terminal.status, TerminalStatus::Live);
    assert!(matches!(terminal.backend, TerminalBackend::Pty { .. }));
}

#[test]
fn broker_input_reaches_shell_and_output_is_broadcast_to_two_clients() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000702");
    fixture.spawn();
    let mut left = fixture.client();
    let mut right = fixture.client();
    left.send(&ClientRequest::Attach {
        terminal_id: fixture.terminal_id.clone(),
    })
    .expect("attach left");
    right
        .send(&ClientRequest::Attach {
            terminal_id: fixture.terminal_id.clone(),
        })
        .expect("attach right");

    fixture.input("printf broker-broadcast-ok\\n\r");

    assert_output_contains(&mut left, "broker-broadcast-ok");
    assert_output_contains(&mut right, "broker-broadcast-ok");
}

#[test]
fn broker_resize_updates_pty_and_snapshot_dimensions() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000703");
    fixture.spawn();
    let mut client = fixture.client();

    assert!(matches!(
        client
            .request(&ClientRequest::Resize {
                terminal_id: TerminalId::parse("00000000-0000-0000-0000-000000000703")
                    .expect("terminal id"),
                cols: 100,
                rows: 30,
            })
            .expect("resize"),
        ServerEvent::Ack { .. }
    ));

    assert!(matches!(
        client
            .request(&ClientRequest::Capture {
                terminal_id: TerminalId::parse("00000000-0000-0000-0000-000000000703")
                    .expect("terminal id"),
                lines: None,
            })
            .expect("capture"),
        ServerEvent::Snapshot {
            cols: 100,
            rows: 30,
            ..
        }
    ));
}

#[test]
fn broker_capture_returns_vt100_screen_contents() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000704");
    fixture.spawn();
    fixture.input("printf broker-capture-ok\\n\r");

    let snapshot = fixture.capture_until("broker-capture-ok");
    assert!(snapshot.contains("broker-capture-ok"));
}

#[test]
fn broker_detach_preserves_session_until_terminate() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000705");
    fixture.spawn();
    let mut client = fixture.client();

    client
        .request(&ClientRequest::Detach {
            terminal_id: fixture.terminal_id.clone(),
        })
        .expect("detach");
    assert_eq!(fixture.session_status(), "live");

    client
        .request(&ClientRequest::Terminate {
            terminal_id: fixture.terminal_id.clone(),
        })
        .expect("terminate");
    assert!(!fixture.session_is_listed());
}

#[test]
fn broker_terminate_marks_registry_exited_stops_child_and_removes_session_from_list() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000706");
    let spawned = fixture.spawn();
    let mut client = fixture.client();

    client
        .request(&ClientRequest::Terminate {
            terminal_id: fixture.terminal_id.clone(),
        })
        .expect("terminate");

    let registry = RegistryStore::new(fixture.config.registry_path.clone())
        .load()
        .expect("registry");
    let terminal = registry
        .terminal(&fixture.terminal_id)
        .expect("registered terminal");
    assert_eq!(terminal.status, TerminalStatus::Exited);
    assert!(!fixture.session_is_listed());
    assert_process_exited(spawned.process_id.expect("spawn pid"));
}

#[test]
fn broker_removes_session_from_list_when_shell_exits() {
    let fixture = BrokerFixture::start("00000000-0000-0000-0000-000000000707");
    fixture.spawn();

    fixture.input("exit\n");

    assert_session_removed_from_list(&fixture);
}
