use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrokerConfig {
    pub registry_path: PathBuf,
    pub socket_path: PathBuf,
    pub ready_timeout: Duration,
}

impl BrokerConfig {
    pub const fn new(
        registry_path: PathBuf,
        socket_path: PathBuf,
        ready_timeout: Duration,
    ) -> Self {
        Self {
            registry_path,
            socket_path,
            ready_timeout,
        }
    }

    pub fn pid_path(&self) -> PathBuf {
        pid_path_for_socket(&self.socket_path)
    }

    pub fn log_path(&self) -> PathBuf {
        log_path_for_socket(&self.socket_path)
    }
}

pub fn default_state_dir() -> PathBuf {
    env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("sexy-terminal-panel")
}

pub fn default_socket_path() -> PathBuf {
    default_state_dir().join("broker.sock")
}

pub fn pid_path_for_socket(socket_path: &Path) -> PathBuf {
    socket_path.with_extension("pid")
}

pub fn log_path_for_socket(socket_path: &Path) -> PathBuf {
    socket_path.with_extension("log")
}
