#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(dead_code)]

use std::fs;
use std::time::Duration;

use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use stp_core::ids::{TerminalId, WindowId};
use stp_core::protocol::{ClientRequest, ServerEvent};
use stp_pty::{BrokerClient, BrokerConfig, RunningBroker};
use tempfile::TempDir;

#[derive(Debug)]
pub struct BrokerFixture {
    temp: TempDir,
    pub config: BrokerConfig,
    pub terminal_id: TerminalId,
    window_id: WindowId,
    _broker: RunningBroker,
}

impl BrokerFixture {
    pub fn start(terminal_id: &str) -> Self {
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

    pub fn client(&self) -> BrokerClient {
        BrokerClient::connect(&self.config.socket_path).expect("client")
    }

    pub fn spawn(&self) -> Spawned {
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

    pub fn input(&self, command: &str) {
        let mut client = self.client();
        client
            .request(&ClientRequest::input_bytes(
                self.terminal_id.clone(),
                command.as_bytes(),
            ))
            .expect("input");
    }

    pub fn capture_until(&self, needle: &str) -> String {
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

    pub fn session_status(&self) -> String {
        self.listed_session_status().unwrap_or_default()
    }

    pub fn session_is_listed(&self) -> bool {
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

#[derive(Debug)]
pub struct Spawned {
    _terminal_id: TerminalId,
    pub process_id: Option<u32>,
}

pub fn assert_output_contains(client: &mut BrokerClient, needle: &str) {
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

pub fn assert_process_exited(pid: u32) {
    for _ in 0..20 {
        if !process_exists(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!process_exists(pid), "process {pid} still exists");
}

pub fn assert_session_removed_from_list(fixture: &BrokerFixture) {
    for _ in 0..20 {
        if !fixture.session_is_listed() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !fixture.session_is_listed(),
        "session still listed after exit"
    );
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
