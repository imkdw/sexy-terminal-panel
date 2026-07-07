#![allow(clippy::expect_used)]

use std::fs;
use std::time::Duration;

use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use stp_core::ids::{TerminalId, WindowId};
use stp_core::protocol::{ClientRequest, ServerEvent};
use stp_core::registry::{RegistryStore, TerminalBackend, TerminalStatus};
use stp_pty::{BrokerClient, BrokerConfig, RunningBroker};
use tempfile::TempDir;

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

struct BrokerFixture {
    temp: TempDir,
    config: BrokerConfig,
    terminal_id: TerminalId,
    window_id: WindowId,
    _broker: RunningBroker,
}

impl BrokerFixture {
    fn start(terminal_id: &str) -> Self {
        let temp = TempDir::new().expect("temp dir");
        let config = BrokerConfig::new(
            temp.path().join("registry.json"),
            temp.path().join("stp.sock"),
            Duration::from_secs(3),
        );
        let broker = RunningBroker::start(config.clone()).expect("broker");
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace).expect("workspace");
        Self {
            temp,
            config,
            terminal_id: TerminalId::parse(terminal_id).expect("terminal id"),
            window_id: WindowId::parse("00000000-0000-0000-0000-000000000601").expect("window id"),
            _broker: broker,
        }
    }

    fn client(&self) -> BrokerClient {
        BrokerClient::connect(&self.config.socket_path).expect("client")
    }

    fn spawn(&self) -> Spawned {
        let mut client = self.client();
        let event = client
            .request(&ClientRequest::Spawn {
                terminal_id: self.terminal_id.clone(),
                window_id: self.window_id.clone(),
                workspace_path: self.temp.path().join("workspace"),
                shell: Some("sh".to_owned()),
            })
            .expect("spawn");
        assert!(
            matches!(event, ServerEvent::Spawned { .. }),
            "unexpected spawn event: {event:?}"
        );
        if let ServerEvent::Spawned {
            terminal_id,
            process_id,
        } = event
        {
            return Spawned {
                _terminal_id: terminal_id,
                process_id,
            };
        }
        Spawned {
            _terminal_id: self.terminal_id.clone(),
            process_id: None,
        }
    }

    fn input(&self, command: &str) {
        let mut client = self.client();
        client
            .request(&ClientRequest::input_bytes(
                self.terminal_id.clone(),
                command.as_bytes(),
            ))
            .expect("input");
    }

    fn capture_until(&self, needle: &str) -> String {
        for _ in 0..20 {
            let mut client = self.client();
            match client
                .request(&ClientRequest::Capture {
                    terminal_id: self.terminal_id.clone(),
                    lines: None,
                })
                .expect("capture")
            {
                ServerEvent::Snapshot { text, .. } if text.contains(needle) => return text,
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        String::new()
    }

    fn session_status(&self) -> String {
        self.listed_session_status().unwrap_or_default()
    }

    fn session_is_listed(&self) -> bool {
        self.listed_session_status().is_some()
    }

    fn listed_session_status(&self) -> Option<String> {
        let mut client = self.client();
        let event = client.request(&ClientRequest::List).expect("list");
        assert!(
            matches!(event, ServerEvent::SessionList { .. }),
            "unexpected list event: {event:?}"
        );
        match event {
            ServerEvent::SessionList { sessions } => sessions
                .into_iter()
                .find(|session| session.terminal_id == self.terminal_id)
                .map(|session| session.status),
            _ => None,
        }
    }
}

struct Spawned {
    _terminal_id: TerminalId,
    process_id: Option<u32>,
}

fn assert_output_contains(client: &mut BrokerClient, needle: &str) {
    let mut found = false;
    for _ in 0..40 {
        if let Some(ServerEvent::Output { data_base64, .. }) = client
            .read_event_timeout(Duration::from_millis(100))
            .expect("read")
        {
            let data =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_base64)
                    .expect("output base64");
            let text = String::from_utf8_lossy(&data);
            if text.contains(needle) {
                found = true;
                break;
            }
        }
    }
    assert!(found, "missing output containing {needle}");
}

fn assert_process_exited(pid: u32) {
    for _ in 0..20 {
        if !process_exists(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!process_exists(pid), "process {pid} still exists");
}

fn assert_session_removed_from_list(fixture: &BrokerFixture) {
    for _ in 0..20 {
        if !fixture.session_is_listed() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!fixture.session_is_listed(), "session still listed after exit");
}

fn process_exists(pid: u32) -> bool {
    let Ok(raw_pid) = i32::try_from(pid) else {
        return false;
    };
    matches!(
        kill(Pid::from_raw(raw_pid), None),
        Ok(()) | Err(Errno::EPERM)
    )
}
