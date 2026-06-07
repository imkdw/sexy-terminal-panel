use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionEndpoint {
    pub socket_path: PathBuf,
}

impl SessionEndpoint {
    pub const fn unix_socket(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub fn is_empty(&self) -> bool {
        self.socket_path.as_os_str().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TerminalBackend {
    Pty {
        endpoint: SessionEndpoint,
    },
    LegacyTmux {
        socket: String,
        session: String,
        #[serde(default = "default_tmux_window")]
        window: String,
    },
}

impl TerminalBackend {
    pub const fn legacy_tmux(socket: String, session: String, window: String) -> Self {
        Self::LegacyTmux {
            socket,
            session,
            window,
        }
    }
}

pub fn default_tmux_window() -> String {
    "0".to_owned()
}
