use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};

use portable_pty::{Child, MasterPty};
use stp_core::ids::{TerminalId, WindowId};
use stp_core::protocol::{ClientRequest, ServerEvent, SessionSummary};
use stp_core::registry::{ManagedTerminal, RegistryStore, SessionEndpoint, TerminalStatus};

use crate::config::BrokerConfig;
use crate::error::BrokerError;

pub const DEFAULT_ROWS: u16 = 24;
pub const DEFAULT_COLS: u16 = 80;
pub const SCROLLBACK_LINES: usize = 2_000;

pub struct BrokerState {
    config: BrokerConfig,
    server_id: String,
    shutdown: Arc<AtomicBool>,
    sessions: Mutex<HashMap<TerminalId, Arc<BrokerSession>>>,
}

impl BrokerState {
    pub fn new(config: BrokerConfig, shutdown: Arc<AtomicBool>) -> Self {
        let server_id = format!("stp-broker-{}", std::process::id());
        Self {
            config,
            server_id,
            shutdown,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    pub fn handle_request(
        &self,
        request: ClientRequest,
        client_tx: Sender<ServerEvent>,
    ) -> Result<ServerEvent, BrokerError> {
        match request {
            ClientRequest::Hello { .. } => Ok(ServerEvent::HelloAck {
                server_id: self.server_id.clone(),
            }),
            ClientRequest::Status => Ok(ServerEvent::Status {
                server_id: self.server_id.clone(),
                pid: std::process::id(),
                sessions: self.session_summaries()?,
            }),
            ClientRequest::List => Ok(ServerEvent::SessionList {
                sessions: self.session_summaries()?,
            }),
            ClientRequest::Spawn {
                terminal_id,
                window_id,
                workspace_path,
                shell,
            } => self.spawn(terminal_id, window_id, &workspace_path, shell),
            ClientRequest::Attach { terminal_id } => self.attach(&terminal_id, client_tx),
            ClientRequest::Input {
                terminal_id,
                data_base64,
            } => {
                let input = ClientRequest::Input {
                    terminal_id: terminal_id.clone(),
                    data_base64,
                };
                self.input(&terminal_id, &input.input_data()?)
            }
            ClientRequest::Resize {
                terminal_id,
                cols,
                rows,
            } => self.resize(&terminal_id, cols, rows),
            ClientRequest::Capture { terminal_id, lines } => self.capture(&terminal_id, lines),
            ClientRequest::Terminate { terminal_id } => self.terminate(&terminal_id),
            ClientRequest::Detach { terminal_id } => self.detach(&terminal_id),
            ClientRequest::Shutdown => {
                self.shutdown.store(true, Ordering::SeqCst);
                self.terminate_all()?;
                Ok(ServerEvent::Ack {
                    message: "stopped broker".to_owned(),
                })
            }
        }
    }

    pub fn terminate_all(&self) -> Result<(), BrokerError> {
        for session in lock(&self.sessions, "sessions")?.values() {
            if session.status()? != TerminalStatus::Exited {
                session.terminate()?;
            }
        }
        Ok(())
    }

    fn spawn(
        &self,
        terminal_id: TerminalId,
        window_id: WindowId,
        workspace_path: &Path,
        shell: Option<String>,
    ) -> Result<ServerEvent, BrokerError> {
        let endpoint = SessionEndpoint::unix_socket(self.config.socket_path.clone());
        let terminal =
            ManagedTerminal::new_pty(terminal_id.clone(), window_id, workspace_path, endpoint)?;
        let session = BrokerSession::spawn(terminal_id.clone(), workspace_path, shell)?;
        let process_id = session.process_id;
        lock(&self.sessions, "sessions")?.insert(terminal_id.clone(), session);
        let store = RegistryStore::new(self.config.registry_path.clone());
        let mut registry = store.load()?;
        registry.upsert(terminal);
        store.save(&registry)?;
        Ok(ServerEvent::Spawned {
            terminal_id,
            process_id,
        })
    }

    fn attach(
        &self,
        terminal_id: &TerminalId,
        client_tx: Sender<ServerEvent>,
    ) -> Result<ServerEvent, BrokerError> {
        let session = self.session(terminal_id)?;
        session.add_subscriber(client_tx)?;
        session.snapshot(None)
    }

    fn input(&self, terminal_id: &TerminalId, data: &[u8]) -> Result<ServerEvent, BrokerError> {
        self.session(terminal_id)?.input(data)?;
        Ok(ServerEvent::Ack {
            message: "input accepted".to_owned(),
        })
    }

    fn resize(
        &self,
        terminal_id: &TerminalId,
        cols: u16,
        rows: u16,
    ) -> Result<ServerEvent, BrokerError> {
        self.session(terminal_id)?.resize(cols, rows)?;
        Ok(ServerEvent::Ack {
            message: "resized".to_owned(),
        })
    }

    fn capture(
        &self,
        terminal_id: &TerminalId,
        lines: Option<u16>,
    ) -> Result<ServerEvent, BrokerError> {
        self.session(terminal_id)?.snapshot(lines)
    }

    fn terminate(&self, terminal_id: &TerminalId) -> Result<ServerEvent, BrokerError> {
        self.session(terminal_id)?.terminate()?;
        self.remove_exited_sessions(std::slice::from_ref(terminal_id))?;
        Ok(ServerEvent::Ack {
            message: "terminated".to_owned(),
        })
    }

    fn detach(&self, terminal_id: &TerminalId) -> Result<ServerEvent, BrokerError> {
        self.session(terminal_id)?;
        Ok(ServerEvent::Ack {
            message: "detached".to_owned(),
        })
    }

    fn session(&self, terminal_id: &TerminalId) -> Result<Arc<BrokerSession>, BrokerError> {
        lock(&self.sessions, "sessions")?
            .get(terminal_id)
            .cloned()
            .ok_or_else(|| BrokerError::TerminalNotFound(terminal_id.clone()))
    }

    fn session_summaries(&self) -> Result<Vec<SessionSummary>, BrokerError> {
        let sessions = lock(&self.sessions, "sessions")?;
        let mut summaries = Vec::with_capacity(sessions.len());
        let mut exited = Vec::new();
        for (terminal_id, session) in sessions.iter() {
            let status = session.status()?;
            if status == TerminalStatus::Exited {
                exited.push(terminal_id.clone());
                continue;
            }
            summaries.push(SessionSummary {
                terminal_id: terminal_id.clone(),
                status: status_label(status),
            });
        }
        drop(sessions);
        self.remove_exited_sessions(&exited)?;
        Ok(summaries)
    }

    fn remove_exited_sessions(&self, terminal_ids: &[TerminalId]) -> Result<(), BrokerError> {
        if terminal_ids.is_empty() {
            return Ok(());
        }
        for terminal_id in terminal_ids {
            self.update_registry_status(terminal_id, TerminalStatus::Exited)?;
        }
        let mut sessions = lock(&self.sessions, "sessions")?;
        for terminal_id in terminal_ids {
            sessions.remove(terminal_id);
        }
        Ok(())
    }

    fn update_registry_status(
        &self,
        terminal_id: &TerminalId,
        status: TerminalStatus,
    ) -> Result<(), BrokerError> {
        let store = RegistryStore::new(self.config.registry_path.clone());
        let mut registry = store.load()?;
        if let Some(terminal) = registry
            .terminals
            .iter_mut()
            .find(|terminal| terminal.terminal_id == *terminal_id)
        {
            terminal.status = status;
        }
        store.save(&registry)?;
        Ok(())
    }
}

pub struct BrokerSession {
    pub terminal_id: TerminalId,
    pub process_id: Option<u32>,
    pub seq: AtomicU64,
    pub master: Mutex<Box<dyn MasterPty + Send>>,
    pub writer: Mutex<Box<dyn Write + Send>>,
    pub child: Mutex<Box<dyn Child + Send + Sync>>,
    pub parser: Mutex<vt100::Parser>,
    pub status: Mutex<TerminalStatus>,
    pub subscribers: Mutex<Vec<Sender<ServerEvent>>>,
}

pub fn lock<'a, T>(
    mutex: &'a Mutex<T>,
    name: &'static str,
) -> Result<MutexGuard<'a, T>, BrokerError> {
    mutex.lock().map_err(|_| BrokerError::LockPoisoned(name))
}

pub fn status_label(status: TerminalStatus) -> String {
    match status {
        TerminalStatus::Starting => "starting",
        TerminalStatus::Live => "live",
        TerminalStatus::Detached => "detached",
        TerminalStatus::Stale => "stale",
        TerminalStatus::Exited => "exited",
    }
    .to_owned()
}
