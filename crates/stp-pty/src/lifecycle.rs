use std::fs::{self, OpenOptions};
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use stp_core::protocol::{ClientRequest, ServerEvent, SessionSummary};

use crate::client::BrokerClient;
use crate::config::BrokerConfig;
use crate::error::BrokerError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrokerStatus {
    pub server_id: String,
    pub pid: u32,
    pub sessions: Vec<SessionSummary>,
}

pub fn broker_status(socket_path: &Path) -> Result<BrokerStatus, BrokerError> {
    let mut client = BrokerClient::connect(socket_path)?;
    match client.request(&ClientRequest::Status)? {
        ServerEvent::Status {
            server_id,
            pid,
            sessions,
        } => Ok(BrokerStatus {
            server_id,
            pid,
            sessions,
        }),
        event => Err(BrokerError::UnexpectedEvent(event)),
    }
}

pub fn stop_broker(socket_path: &Path) -> Result<(), BrokerError> {
    let mut client = BrokerClient::connect(socket_path)?;
    match client.request(&ClientRequest::Shutdown)? {
        ServerEvent::Ack { .. } => Ok(()),
        event => Err(BrokerError::UnexpectedEvent(event)),
    }
}

pub fn ensure_broker(
    config: &BrokerConfig,
    executable: &Path,
) -> Result<BrokerStatus, BrokerError> {
    if let Ok(status) = broker_status(&config.socket_path) {
        return Ok(status);
    }
    remove_stale_socket_files(config)?;
    spawn_detached(config, executable)?;
    wait_for_status(&config.socket_path, config.ready_timeout)
}

pub fn wait_for_status(socket_path: &Path, timeout: Duration) -> Result<BrokerStatus, BrokerError> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(status) = broker_status(socket_path) {
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(BrokerError::ReadyTimeout {
        socket: socket_path.to_path_buf(),
    })
}

pub fn remove_stale_socket_files(config: &BrokerConfig) -> Result<(), BrokerError> {
    if broker_status(&config.socket_path).is_ok() {
        return Ok(());
    }
    let Some(stale_pid) = stale_broker_pid(config)? else {
        return Ok(());
    };
    if pid_is_running(stale_pid, &config.pid_path())? {
        return Err(BrokerError::UnsafeSocketPath {
            path: config.socket_path.clone(),
        });
    }
    remove_socket_if_exists(&config.socket_path)?;
    remove_broker_file_if_exists(&config.pid_path())?;
    Ok(())
}

pub fn cleanup_socket_files(config: &BrokerConfig) -> Result<(), BrokerError> {
    remove_socket_if_exists(&config.socket_path)?;
    remove_broker_file_if_exists(&config.pid_path())?;
    Ok(())
}

fn spawn_detached(config: &BrokerConfig, executable: &Path) -> Result<(), BrokerError> {
    if let Some(parent) = config.log_path().parent() {
        fs::create_dir_all(parent)?;
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.log_path())?;
    let stderr = stdout.try_clone()?;
    Command::new(executable)
        .arg("broker")
        .arg("serve")
        .arg("--registry")
        .arg(&config.registry_path)
        .arg("--socket")
        .arg(&config.socket_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;
    Ok(())
}

fn stale_broker_pid(config: &BrokerConfig) -> Result<Option<u32>, BrokerError> {
    let socket_metadata = match fs::symlink_metadata(&config.socket_path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(BrokerError::Io(source)),
    };
    if !socket_metadata.file_type().is_socket() {
        return Err(BrokerError::UnsafeSocketPath {
            path: config.socket_path.clone(),
        });
    }
    read_pid_file(&config.pid_path()).map(Some)
}

fn read_pid_file(path: &Path) -> Result<u32, BrokerError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(BrokerError::UnsafeBrokerFile {
                path: path.to_path_buf(),
            });
        }
        Err(source) => return Err(BrokerError::Io(source)),
    };
    if !metadata.file_type().is_file() {
        return Err(BrokerError::UnsafeBrokerFile {
            path: path.to_path_buf(),
        });
    }
    let raw = fs::read_to_string(path).map_err(BrokerError::Io)?;
    raw.trim()
        .parse::<u32>()
        .map_err(|_| BrokerError::InvalidPidFile {
            path: path.to_path_buf(),
        })
}

fn pid_is_running(pid: u32, pid_path: &Path) -> Result<bool, BrokerError> {
    let raw_pid = i32::try_from(pid).map_err(|_| BrokerError::InvalidPidFile {
        path: pid_path.to_path_buf(),
    })?;
    match kill(Pid::from_raw(raw_pid), None) {
        Ok(()) | Err(Errno::EPERM) => Ok(true),
        Err(Errno::ESRCH) => Ok(false),
        Err(source) => Err(BrokerError::Io(std::io::Error::from(source))),
    }
}

fn remove_socket_if_exists(path: &Path) -> Result<(), BrokerError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(BrokerError::Io(source)),
    };
    if !metadata.file_type().is_socket() {
        return Err(BrokerError::UnsafeSocketPath {
            path: path.to_path_buf(),
        });
    }
    remove_file_if_exists(path)
}

fn remove_broker_file_if_exists(path: &Path) -> Result<(), BrokerError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(BrokerError::Io(source)),
    };
    if !metadata.file_type().is_file() {
        return Err(BrokerError::UnsafeBrokerFile {
            path: path.to_path_buf(),
        });
    }
    remove_file_if_exists(path)
}

fn remove_file_if_exists(path: &Path) -> Result<(), BrokerError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(BrokerError::Io(source)),
    }
}
