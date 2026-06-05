use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{TerminalId, WindowId, WorkspaceId};
use crate::workspace::{WorkspaceError, discover_workspace};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalStatus {
    Starting,
    #[default]
    Live,
    Stale,
    Exited,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManagedTerminal {
    pub terminal_id: TerminalId,
    pub workspace_id: WorkspaceId,
    pub window_id: WindowId,
    pub workspace_path: PathBuf,
    pub repo_root: PathBuf,
    pub branch_name: Option<String>,
    pub tmux_socket: String,
    pub tmux_session: String,
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
        let metadata = discover_workspace(workspace_path)?;
        let now = now_seconds();
        Ok(Self {
            terminal_id,
            workspace_id: metadata.workspace_id,
            window_id,
            workspace_path: metadata.workspace_path,
            repo_root: metadata.repo_root,
            branch_name: metadata.branch_name,
            tmux_socket: tmux_socket.to_owned(),
            tmux_session: tmux_session.to_owned(),
            tmux_window: "0".to_owned(),
            created_at: now,
            last_seen_at: now,
            status: TerminalStatus::Live,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryStore {
    path: PathBuf,
}

impl RegistryStore {
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Registry, RegistryError> {
        if !self.path.exists() {
            return Ok(Registry::default());
        }
        let raw = fs::read_to_string(&self.path).map_err(|source| RegistryError::Read {
            path: self.path.clone(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| RegistryError::MalformedRegistry {
            path: self.path.clone(),
            source,
        })
    }

    pub fn save(&self, registry: &Registry) -> Result<(), RegistryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| RegistryError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let temp_path = self
            .path
            .with_extension(format!("tmp-{}", std::process::id()));
        let write_result = write_atomic(&temp_path, &self.path, registry);
        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        write_result
    }
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error("failed to read registry {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write registry {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("malformed registry {path}: {source}")]
    MalformedRegistry {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to encode registry: {0}")]
    Encode(serde_json::Error),
}

fn write_atomic(
    temp_path: &Path,
    final_path: &Path,
    registry: &Registry,
) -> Result<(), RegistryError> {
    let encoded = serde_json::to_vec_pretty(registry).map_err(RegistryError::Encode)?;
    let mut file = File::create(temp_path).map_err(|source| RegistryError::Write {
        path: temp_path.to_path_buf(),
        source,
    })?;
    file.write_all(&encoded)
        .map_err(|source| RegistryError::Write {
            path: temp_path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| RegistryError::Write {
        path: temp_path.to_path_buf(),
        source,
    })?;
    fs::rename(temp_path, final_path).map_err(|source| RegistryError::Write {
        path: final_path.to_path_buf(),
        source,
    })
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
