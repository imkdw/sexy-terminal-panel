use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::codec::{decode_base64, encode_base64};
use super::error::ProtocolError;
use crate::ids::{TerminalId, WindowId};

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRequest {
    Hello {
        client_id: String,
    },
    Spawn {
        terminal_id: TerminalId,
        window_id: WindowId,
        workspace_path: PathBuf,
        shell: Option<String>,
        /// 지정 시 shell 대신 이 argv 로 PTY 를 띄운다(예: 기존 tmux 세션 attach 래핑).
        #[serde(default)]
        command: Option<Vec<String>>,
    },
    Attach {
        terminal_id: TerminalId,
    },
    Input {
        terminal_id: TerminalId,
        data_base64: String,
    },
    Resize {
        terminal_id: TerminalId,
        cols: u16,
        rows: u16,
    },
    Capture {
        terminal_id: TerminalId,
        lines: Option<u16>,
    },
    Terminate {
        terminal_id: TerminalId,
    },
    Detach {
        terminal_id: TerminalId,
    },
    Shutdown,
    List,
    Status,
}

impl ClientRequest {
    pub fn input_bytes(terminal_id: TerminalId, data: &[u8]) -> Self {
        Self::Input {
            terminal_id,
            data_base64: encode_base64(data),
        }
    }

    pub fn input_data(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Self::Input { data_base64, .. } => decode_base64(data_base64),
            Self::Hello { .. }
            | Self::Spawn { .. }
            | Self::Attach { .. }
            | Self::Resize { .. }
            | Self::Capture { .. }
            | Self::Terminate { .. }
            | Self::Detach { .. }
            | Self::Shutdown
            | Self::List
            | Self::Status => Ok(Vec::new()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Ack {
        message: String,
    },
    HelloAck {
        server_id: String,
    },
    Status {
        server_id: String,
        pid: u32,
        sessions: Vec<SessionSummary>,
    },
    SessionList {
        sessions: Vec<SessionSummary>,
    },
    Spawned {
        terminal_id: TerminalId,
        process_id: Option<u32>,
    },
    Snapshot {
        terminal_id: TerminalId,
        cols: u16,
        rows: u16,
        text: String,
    },
    Output {
        terminal_id: TerminalId,
        seq: u64,
        data_base64: String,
    },
    Exit {
        terminal_id: TerminalId,
        exit_code: Option<i32>,
    },
    Error {
        code: String,
        message: String,
    },
}

impl ServerEvent {
    pub fn output_bytes(terminal_id: TerminalId, seq: u64, data: &[u8]) -> Self {
        Self::Output {
            terminal_id,
            seq,
            data_base64: encode_base64(data),
        }
    }

    pub fn output_data(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Self::Output { data_base64, .. } => decode_base64(data_base64),
            Self::Ack { .. }
            | Self::HelloAck { .. }
            | Self::Status { .. }
            | Self::SessionList { .. }
            | Self::Spawned { .. }
            | Self::Snapshot { .. }
            | Self::Exit { .. }
            | Self::Error { .. } => Ok(Vec::new()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub terminal_id: TerminalId,
    pub status: String,
}
