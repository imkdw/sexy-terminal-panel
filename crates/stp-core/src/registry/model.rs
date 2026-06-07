use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use super::backend::{SessionEndpoint, TerminalBackend, default_tmux_window};
use super::store::RegistryError;
use crate::ids::{TerminalId, WindowId, WorkspaceId};
use crate::workspace::discover_workspace;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalStatus {
    Starting,
    #[default]
    Live,
    Detached,
    Stale,
    Exited,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ManagedTerminal {
    pub terminal_id: TerminalId,
    pub workspace_id: WorkspaceId,
    pub window_id: WindowId,
    pub workspace_path: PathBuf,
    pub repo_root: PathBuf,
    pub branch_name: Option<String>,
    pub backend: TerminalBackend,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tmux_socket: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tmux_session: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tmux_window: String,
    pub created_at: u64,
    pub last_seen_at: u64,
    #[serde(default)]
    pub status: TerminalStatus,
}

impl ManagedTerminal {
    pub fn new(
        terminal_id: TerminalId,
        window_id: WindowId,
        workspace_path: &Path,
        tmux_socket: &str,
        tmux_session: &str,
    ) -> Result<Self, RegistryError> {
        let backend = TerminalBackend::legacy_tmux(
            tmux_socket.to_owned(),
            tmux_session.to_owned(),
            default_tmux_window(),
        );
        Self::from_backend(terminal_id, window_id, workspace_path, backend)
    }

    pub fn new_pty(
        terminal_id: TerminalId,
        window_id: WindowId,
        workspace_path: &Path,
        endpoint: SessionEndpoint,
    ) -> Result<Self, RegistryError> {
        Self::from_backend(
            terminal_id,
            window_id,
            workspace_path,
            TerminalBackend::Pty { endpoint },
        )
    }

    fn from_backend(
        terminal_id: TerminalId,
        window_id: WindowId,
        workspace_path: &Path,
        backend: TerminalBackend,
    ) -> Result<Self, RegistryError> {
        let metadata = discover_workspace(workspace_path)?;
        let now = now_seconds();
        let (tmux_socket, tmux_session, tmux_window) = legacy_fields(&backend);
        Ok(Self {
            terminal_id,
            workspace_id: metadata.workspace_id,
            window_id,
            workspace_path: metadata.workspace_path,
            repo_root: metadata.repo_root,
            branch_name: metadata.branch_name,
            backend,
            tmux_socket,
            tmux_session,
            tmux_window,
            created_at: now,
            last_seen_at: now,
            status: TerminalStatus::Live,
        })
    }
}

impl<'de> Deserialize<'de> for ManagedTerminal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = ManagedTerminalWire::deserialize(deserializer)?;
        let backend = raw.backend.unwrap_or_else(|| {
            TerminalBackend::legacy_tmux(
                raw.tmux_socket.clone().unwrap_or_default(),
                raw.tmux_session.clone().unwrap_or_default(),
                raw.tmux_window.clone().unwrap_or_else(default_tmux_window),
            )
        });
        reject_invalid_live_pty(&backend, raw.status)?;
        let (tmux_socket, tmux_session, tmux_window) = legacy_fields(&backend);
        Ok(Self {
            terminal_id: raw.terminal_id,
            workspace_id: raw.workspace_id,
            window_id: raw.window_id,
            workspace_path: raw.workspace_path,
            repo_root: raw.repo_root,
            branch_name: raw.branch_name,
            backend,
            tmux_socket,
            tmux_session,
            tmux_window,
            created_at: raw.created_at,
            last_seen_at: raw.last_seen_at,
            status: raw.status,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub terminals: Vec<ManagedTerminal>,
}

impl Registry {
    pub fn terminal(&self, terminal_id: &TerminalId) -> Option<&ManagedTerminal> {
        self.terminals
            .iter()
            .find(|terminal| terminal.terminal_id == *terminal_id)
    }

    pub fn upsert(&mut self, terminal: ManagedTerminal) {
        if let Some(existing) = self
            .terminals
            .iter_mut()
            .find(|existing| existing.terminal_id == terminal.terminal_id)
        {
            *existing = terminal;
            return;
        }
        self.terminals.push(terminal);
    }

    pub fn remove_stale(&mut self) -> usize {
        let before = self.terminals.len();
        self.terminals
            .retain(|terminal| terminal.status != TerminalStatus::Stale);
        before.saturating_sub(self.terminals.len())
    }
}

#[derive(Deserialize)]
struct ManagedTerminalWire {
    terminal_id: TerminalId,
    workspace_id: WorkspaceId,
    window_id: WindowId,
    workspace_path: PathBuf,
    repo_root: PathBuf,
    branch_name: Option<String>,
    backend: Option<TerminalBackend>,
    tmux_socket: Option<String>,
    tmux_session: Option<String>,
    tmux_window: Option<String>,
    created_at: u64,
    last_seen_at: u64,
    #[serde(default)]
    status: TerminalStatus,
}

fn reject_invalid_live_pty<E>(backend: &TerminalBackend, status: TerminalStatus) -> Result<(), E>
where
    E: DeError,
{
    match backend {
        TerminalBackend::Pty { endpoint }
            if status == TerminalStatus::Live && endpoint.is_empty() =>
        {
            Err(E::custom(
                "live pty terminal requires a broker socket endpoint",
            ))
        }
        TerminalBackend::Pty { .. } | TerminalBackend::LegacyTmux { .. } => Ok(()),
    }
}

fn legacy_fields(backend: &TerminalBackend) -> (String, String, String) {
    match backend {
        TerminalBackend::LegacyTmux {
            socket,
            session,
            window,
        } => (socket.clone(), session.clone(), window.clone()),
        TerminalBackend::Pty { .. } => (String::new(), String::new(), String::new()),
    }
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
