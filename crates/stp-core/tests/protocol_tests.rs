#![allow(clippy::expect_used)]

use std::path::PathBuf;

use stp_core::ids::{TerminalId, WindowId};
use stp_core::protocol::{
    ClientRequest, MAX_FRAME_BYTES, ProtocolError, ServerEvent, decode_client_frame,
    decode_server_frame, encode_client_frame, encode_server_frame,
};

#[test]
fn protocol_round_trips_spawn_attach_output_and_resize_frames() {
    let terminal_id =
        TerminalId::parse("00000000-0000-0000-0000-000000000501").expect("terminal id");
    let window_id = WindowId::parse("00000000-0000-0000-0000-000000000401").expect("window id");
    let spawn = ClientRequest::Spawn {
        terminal_id: terminal_id.clone(),
        window_id,
        workspace_path: PathBuf::from("/tmp/worktree-a"),
        shell: Some("sh".to_owned()),
    };

    let decoded_spawn =
        decode_client_frame(&encode_client_frame(&spawn).expect("encode spawn")).expect("spawn");
    let decoded_attach = decode_client_frame(
        &encode_client_frame(&ClientRequest::Attach {
            terminal_id: terminal_id.clone(),
        })
        .expect("encode attach"),
    )
    .expect("attach");
    let decoded_resize = decode_client_frame(
        &encode_client_frame(&ClientRequest::Resize {
            terminal_id: terminal_id.clone(),
            cols: 120,
            rows: 40,
        })
        .expect("encode resize"),
    )
    .expect("resize");
    let output = ServerEvent::output_bytes(terminal_id.clone(), 7, b"hello");
    let decoded_output =
        decode_server_frame(&encode_server_frame(&output).expect("encode output")).expect("output");

    assert_eq!(decoded_spawn, spawn);
    assert_eq!(
        decoded_attach,
        ClientRequest::Attach {
            terminal_id: terminal_id.clone(),
        }
    );
    assert_eq!(
        decoded_resize,
        ClientRequest::Resize {
            terminal_id,
            cols: 120,
            rows: 40,
        }
    );
    assert_eq!(
        decoded_output.output_data().expect("output bytes"),
        b"hello"
    );
}

#[test]
fn protocol_preserves_non_utf8_pty_bytes_with_base64() {
    let terminal_id =
        TerminalId::parse("00000000-0000-0000-0000-000000000502").expect("terminal id");
    let bytes = [0, 159, 146, 150, 255];
    let output = ServerEvent::output_bytes(terminal_id, 9, &bytes);

    let decoded =
        decode_server_frame(&encode_server_frame(&output).expect("encode")).expect("decode");

    assert_eq!(decoded.output_data().expect("output bytes"), bytes);
}

#[test]
fn protocol_rejects_unknown_version_and_malformed_base64() {
    let unknown_version = decode_client_frame(r#"{"version":999,"type":"list"}"#)
        .expect_err("unknown version should fail");
    let malformed_base64 = decode_server_frame(
        r#"{"version":1,"type":"output","terminal_id":"00000000-0000-0000-0000-000000000503","seq":1,"data_base64":"%"}"#,
    )
    .expect_err("malformed base64 should fail");

    assert!(matches!(
        unknown_version,
        ProtocolError::UnsupportedVersion { version: 999 }
    ));
    assert!(matches!(
        malformed_base64,
        ProtocolError::MalformedBase64 { .. }
    ));
}

#[test]
fn protocol_rejects_oversized_frames_and_payloads() {
    let oversized_frame = " ".repeat(MAX_FRAME_BYTES + 1);
    let oversized_payload = format!(
        r#"{{"version":1,"type":"input","terminal_id":"00000000-0000-0000-0000-000000000504","data_base64":"{}"}}"#,
        "A".repeat(700_000)
    );

    assert!(matches!(
        decode_client_frame(&oversized_frame).expect_err("frame too large"),
        ProtocolError::FrameTooLarge { .. }
    ));
    assert!(matches!(
        decode_client_frame(&oversized_payload).expect_err("payload too large"),
        ProtocolError::PayloadTooLarge { .. }
    ));
}
