#![allow(clippy::expect_used)]

use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

use stp_core::protocol::{ClientRequest, MAX_FRAME_BYTES, ServerEvent, decode_server_frame};
use stp_pty::{BrokerClient, BrokerConfig, BrokerError, RunningBroker, pid_path_for_socket};
use tempfile::TempDir;

#[test]
fn broker_replaces_stale_socket_when_no_server_responds() {
    let temp = TempDir::new().expect("temp dir");
    let config = broker_config(&temp);
    let stale = UnixListener::bind(&config.socket_path).expect("stale socket");
    drop(stale);
    fs::write(pid_path_for_socket(&config.socket_path), "999999").expect("stale pid");

    let broker = RunningBroker::start(config.clone()).expect("broker");
    let mut client = BrokerClient::connect(&config.socket_path).expect("client");

    assert!(matches!(
        client.request(&ClientRequest::Status).expect("status"),
        ServerEvent::Status { .. }
    ));

    broker.stop().expect("stop broker");
    assert!(!config.socket_path.exists());
    assert!(!pid_path_for_socket(&config.socket_path).exists());
}

#[test]
fn broker_refuses_to_replace_regular_file_socket_path() {
    let temp = TempDir::new().expect("temp dir");
    let config = broker_config(&temp);
    fs::write(&config.socket_path, "not a socket").expect("regular file");

    let error = stp_pty::serve(config.clone()).expect_err("regular file should be unsafe");

    assert!(matches!(
        error,
        BrokerError::UnsafeSocketPath { path } if path == config.socket_path
    ));
    assert!(config.socket_path.exists());
}

#[test]
fn broker_refuses_to_replace_unmarked_socket_path() {
    let temp = TempDir::new().expect("temp dir");
    let config = broker_config(&temp);
    let foreign = UnixListener::bind(&config.socket_path).expect("foreign socket");
    drop(foreign);

    let error = stp_pty::serve(config.clone()).expect_err("unmarked socket should be unsafe");

    assert!(matches!(
        error,
        BrokerError::UnsafeBrokerFile { path } if path == pid_path_for_socket(&config.socket_path)
    ));
    assert!(config.socket_path.exists());
}

#[test]
fn broker_malformed_frame_returns_error_and_keeps_server_alive() {
    let temp = TempDir::new().expect("temp dir");
    let config = broker_config(&temp);
    let broker = RunningBroker::start(config.clone()).expect("broker");
    let mut stream = UnixStream::connect(&config.socket_path).expect("connect");

    stream
        .write_all(b"{not-json}\n")
        .expect("write malformed frame");
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut raw = String::new();
    reader.read_line(&mut raw).expect("read error frame");
    assert!(matches!(
        decode_server_frame(&raw).expect("decode error"),
        ServerEvent::Error { code, .. } if code == "malformed_frame"
    ));

    let mut client = BrokerClient::connect(&config.socket_path).expect("client");
    assert!(matches!(
        client.request(&ClientRequest::Status).expect("status"),
        ServerEvent::Status { .. }
    ));

    broker.stop().expect("stop broker");
}

#[test]
fn broker_oversized_frame_returns_error_and_closes_client() {
    let temp = TempDir::new().expect("temp dir");
    let config = broker_config(&temp);
    let broker = RunningBroker::start(config.clone()).expect("broker");
    let mut stream = UnixStream::connect(&config.socket_path).expect("connect");

    match stream.write_all(&vec![b'x'; MAX_FRAME_BYTES + 1]) {
        Ok(()) => stream.shutdown(Shutdown::Write).expect("close write half"),
        Err(source) if source.kind() == ErrorKind::BrokenPipe => {}
        Err(source) => panic!("write oversized frame: {source}"),
    }
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut raw = String::new();
    let bytes = reader.read_line(&mut raw).expect("read error frame");
    if bytes > 0 {
        assert!(matches!(
            decode_server_frame(&raw).expect("decode error"),
            ServerEvent::Error { code, .. } if code == "frame_too_large"
        ));
    }

    let mut client = BrokerClient::connect(&config.socket_path).expect("client");
    assert!(matches!(
        client.request(&ClientRequest::Status).expect("status"),
        ServerEvent::Status { .. }
    ));

    broker.stop().expect("stop broker");
}

fn broker_config(temp: &TempDir) -> BrokerConfig {
    BrokerConfig::new(
        temp.path().join("registry.json"),
        temp.path().join("stp.sock"),
        Duration::from_secs(2),
    )
}
