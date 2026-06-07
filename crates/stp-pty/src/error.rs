use std::path::PathBuf;

use stp_core::ids::TerminalId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("broker io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("broker protocol failed: {0}")]
    Protocol(#[from] stp_core::protocol::ProtocolError),
    #[error("registry operation failed: {0}")]
    Registry(#[from] stp_core::registry::RegistryError),
    #[error("pty operation failed: {0}")]
    Pty(String),
    #[error("broker did not become ready at {socket}")]
    ReadyTimeout { socket: PathBuf },
    #[error("refusing to remove unsafe broker socket path: {path}")]
    UnsafeSocketPath { path: PathBuf },
    #[error("refusing to remove unsafe broker metadata path: {path}")]
    UnsafeBrokerFile { path: PathBuf },
    #[error("broker connection closed")]
    ConnectionClosed,
    #[error("terminal not found: {0}")]
    TerminalNotFound(TerminalId),
    #[error("broker lock poisoned: {0}")]
    LockPoisoned(&'static str),
    #[error("broker thread panicked")]
    ThreadPanicked,
    #[error("unexpected broker event: {0:?}")]
    UnexpectedEvent(stp_core::protocol::ServerEvent),
    #[error("broker pid file is invalid: {path}")]
    InvalidPidFile { path: PathBuf },
}
