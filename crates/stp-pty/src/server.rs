use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use stp_core::protocol::{MAX_FRAME_BYTES, ServerEvent, decode_client_frame, encode_server_frame};

use crate::config::BrokerConfig;
use crate::error::BrokerError;
use crate::lifecycle::{cleanup_socket_files, remove_stale_socket_files, wait_for_status};
use crate::session::BrokerState;

pub struct RunningBroker {
    config: BrokerConfig,
    shutdown: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<Result<(), BrokerError>>>,
}

impl std::fmt::Debug for RunningBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningBroker")
            .field("config", &self.config)
            .field("shutdown", &self.shutdown.load(Ordering::SeqCst))
            .field("has_handle", &self.handle.is_some())
            .finish()
    }
}

impl RunningBroker {
    pub fn start(config: BrokerConfig) -> Result<Self, BrokerError> {
        let thread_config = config.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let handle = thread::Builder::new()
            .name("stp-pty-broker".to_owned())
            .spawn(move || serve_with_shutdown(thread_config, thread_shutdown))?;
        wait_for_status(&config.socket_path, config.ready_timeout)?;
        Ok(Self {
            config,
            shutdown,
            handle: Some(handle),
        })
    }

    pub fn stop(mut self) -> Result<(), BrokerError> {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = crate::lifecycle::stop_broker(&self.config.socket_path);
        self.join()
    }

    fn join(&mut self) -> Result<(), BrokerError> {
        if let Some(handle) = self.handle.take() {
            return handle.join().map_err(|_| BrokerError::ThreadPanicked)?;
        }
        Ok(())
    }
}

impl Drop for RunningBroker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = crate::lifecycle::stop_broker(&self.config.socket_path);
        let _ = self.join();
    }
}

pub fn serve(config: BrokerConfig) -> Result<(), BrokerError> {
    serve_with_shutdown(config, Arc::new(AtomicBool::new(false)))
}

fn serve_with_shutdown(config: BrokerConfig, shutdown: Arc<AtomicBool>) -> Result<(), BrokerError> {
    prepare_socket(&config)?;
    let listener = UnixListener::bind(&config.socket_path)?;
    let _cleanup = SocketCleanup::new(config.clone());
    fs::set_permissions(&config.socket_path, fs::Permissions::from_mode(0o600))?;
    fs::write(config.pid_path(), std::process::id().to_string())?;
    listener.set_nonblocking(true)?;
    let state = Arc::new(BrokerState::new(config, shutdown));
    while !state.is_shutdown() {
        match listener.accept() {
            Ok((stream, _addr)) => spawn_client(stream, Arc::clone(&state))?,
            Err(source) if source.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(source) => return Err(BrokerError::Io(source)),
        }
    }
    state.terminate_all()?;
    Ok(())
}

fn prepare_socket(config: &BrokerConfig) -> Result<(), BrokerError> {
    if let Some(parent) = config.socket_path.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    remove_stale_socket_files(config)
}

fn spawn_client(stream: UnixStream, state: Arc<BrokerState>) -> Result<(), BrokerError> {
    thread::Builder::new()
        .name("stp-pty-client".to_owned())
        .spawn(move || {
            let _ = handle_client(stream, &state);
        })?;
    Ok(())
}

fn handle_client(stream: UnixStream, state: &BrokerState) -> Result<(), BrokerError> {
    // macOS 는 accept 된 소켓이 리스너의 non-blocking 플래그를 상속한다. 그대로 두면
    // 즉시 데이터를 보내지 않는 idle 연결(예: attach 후 대기하는 event 연결)의 read 가
    // WouldBlock 으로 에러 처리되어 연결이 끊긴다. 명시적으로 blocking 으로 되돌린다.
    stream.set_nonblocking(false)?;
    let reader_stream = stream.try_clone()?;
    let mut direct_stream = stream.try_clone()?;
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("stp-pty-writer".to_owned())
        .spawn(move || writer_loop(stream, rx))?;
    let mut reader = BufReader::new(reader_stream);
    loop {
        let frame = read_client_frame(&mut reader)?;
        let event = match frame {
            ClientFrameRead::Closed => break,
            ClientFrameRead::TooLarge => {
                write_server_event(
                    &mut direct_stream,
                    &ServerEvent::Error {
                        code: "frame_too_large".to_owned(),
                        message: format!("protocol frame exceeds {MAX_FRAME_BYTES} bytes"),
                    },
                )?;
                break;
            }
            ClientFrameRead::InvalidUtf8 => ServerEvent::Error {
                code: "malformed_frame".to_owned(),
                message: "protocol frame is not utf-8".to_owned(),
            },
            ClientFrameRead::Frame(raw) => match decode_client_frame(&raw) {
                Ok(request) => state.handle_request(request, tx.clone())?,
                Err(source) => ServerEvent::Error {
                    code: "malformed_frame".to_owned(),
                    message: source.to_string(),
                },
            },
        };
        if tx.send(event).is_err() {
            break;
        }
    }
    Ok(())
}

fn write_server_event(stream: &mut UnixStream, event: &ServerEvent) -> Result<(), BrokerError> {
    let frame = encode_server_frame(event)?;
    stream.write_all(frame.as_bytes())?;
    stream.flush()?;
    Ok(())
}

enum ClientFrameRead {
    Frame(String),
    Closed,
    TooLarge,
    InvalidUtf8,
}

fn read_client_frame(reader: &mut BufReader<UnixStream>) -> Result<ClientFrameRead, BrokerError> {
    let mut raw = Vec::new();
    loop {
        let (consume, complete, too_large) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return Ok(frame_from_bytes(raw));
            }
            let newline = available.iter().position(|byte| *byte == b'\n');
            let available_end = newline.map_or(available.len(), |index| index + 1);
            let remaining = MAX_FRAME_BYTES.saturating_sub(raw.len());
            if available_end > remaining {
                (remaining.saturating_add(1).min(available_end), false, true)
            } else {
                raw.extend_from_slice(&available[..available_end]);
                (available_end, newline.is_some(), false)
            }
        };
        reader.consume(consume);
        if too_large {
            return Ok(ClientFrameRead::TooLarge);
        }
        if complete {
            return Ok(frame_from_bytes(raw));
        }
    }
}

fn frame_from_bytes(raw: Vec<u8>) -> ClientFrameRead {
    if raw.is_empty() {
        return ClientFrameRead::Closed;
    }
    String::from_utf8(raw).map_or(ClientFrameRead::InvalidUtf8, ClientFrameRead::Frame)
}

fn writer_loop(mut stream: UnixStream, rx: Receiver<ServerEvent>) {
    for event in rx {
        let Ok(frame) = encode_server_frame(&event) else {
            break;
        };
        if stream.write_all(frame.as_bytes()).is_err() || stream.flush().is_err() {
            break;
        }
    }
}

struct SocketCleanup {
    config: BrokerConfig,
}

impl SocketCleanup {
    const fn new(config: BrokerConfig) -> Self {
        Self { config }
    }
}

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = cleanup_socket_files(&self.config);
    }
}
