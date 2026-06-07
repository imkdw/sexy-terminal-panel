use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use super::error::ProtocolError;
use super::model::{ClientRequest, PROTOCOL_VERSION, ServerEvent};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_BINARY_PAYLOAD_BYTES: usize = 512 * 1024;
const MAX_BASE64_PAYLOAD_CHARS: usize = MAX_BINARY_PAYLOAD_BYTES.div_ceil(3) * 4;

pub fn encode_client_frame(request: &ClientRequest) -> Result<String, ProtocolError> {
    encode_frame(ClientFrame {
        version: PROTOCOL_VERSION,
        request: request.clone(),
    })
}

pub fn decode_client_frame(raw: &str) -> Result<ClientRequest, ProtocolError> {
    let frame: ClientFrame = decode_frame(raw)?;
    reject_unknown_version(frame.version)?;
    validate_client_request(&frame.request)?;
    Ok(frame.request)
}

pub fn encode_server_frame(event: &ServerEvent) -> Result<String, ProtocolError> {
    encode_frame(ServerFrame {
        version: PROTOCOL_VERSION,
        event: event.clone(),
    })
}

pub fn decode_server_frame(raw: &str) -> Result<ServerEvent, ProtocolError> {
    let frame: ServerFrame = decode_frame(raw)?;
    reject_unknown_version(frame.version)?;
    validate_server_event(&frame.event)?;
    Ok(frame.event)
}

#[derive(Serialize, Deserialize)]
struct ClientFrame {
    version: u16,
    #[serde(flatten)]
    request: ClientRequest,
}

#[derive(Serialize, Deserialize)]
struct ServerFrame {
    version: u16,
    #[serde(flatten)]
    event: ServerEvent,
}

fn encode_frame<T>(frame: T) -> Result<String, ProtocolError>
where
    T: Serialize,
{
    serde_json::to_string(&frame)
        .map(|encoded| format!("{encoded}\n"))
        .map_err(ProtocolError::Encode)
}

fn decode_frame<'a, T>(raw: &'a str) -> Result<T, ProtocolError>
where
    T: Deserialize<'a>,
{
    if raw.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            limit: MAX_FRAME_BYTES,
        });
    }
    serde_json::from_str(raw.trim_end_matches('\n')).map_err(ProtocolError::Decode)
}

const fn reject_unknown_version(version: u16) -> Result<(), ProtocolError> {
    if version == PROTOCOL_VERSION {
        return Ok(());
    }
    Err(ProtocolError::UnsupportedVersion { version })
}

fn validate_client_request(request: &ClientRequest) -> Result<(), ProtocolError> {
    match request {
        ClientRequest::Input { data_base64, .. } => decode_base64(data_base64).map(|_| ()),
        ClientRequest::Hello { .. }
        | ClientRequest::Spawn { .. }
        | ClientRequest::Attach { .. }
        | ClientRequest::Resize { .. }
        | ClientRequest::Capture { .. }
        | ClientRequest::Terminate { .. }
        | ClientRequest::Detach { .. }
        | ClientRequest::Shutdown
        | ClientRequest::List
        | ClientRequest::Status => Ok(()),
    }
}

fn validate_server_event(event: &ServerEvent) -> Result<(), ProtocolError> {
    match event {
        ServerEvent::Output { data_base64, .. } => decode_base64(data_base64).map(|_| ()),
        ServerEvent::Ack { .. }
        | ServerEvent::HelloAck { .. }
        | ServerEvent::Status { .. }
        | ServerEvent::SessionList { .. }
        | ServerEvent::Spawned { .. }
        | ServerEvent::Snapshot { .. }
        | ServerEvent::Exit { .. }
        | ServerEvent::Error { .. } => Ok(()),
    }
}

pub(super) fn encode_base64(data: &[u8]) -> String {
    STANDARD.encode(data)
}

pub(super) fn decode_base64(data: &str) -> Result<Vec<u8>, ProtocolError> {
    if data.len() > MAX_BASE64_PAYLOAD_CHARS {
        return Err(ProtocolError::PayloadTooLarge {
            limit: MAX_BINARY_PAYLOAD_BYTES,
        });
    }
    let decoded = STANDARD
        .decode(data)
        .map_err(|source| ProtocolError::MalformedBase64 { source })?;
    if decoded.len() > MAX_BINARY_PAYLOAD_BYTES {
        return Err(ProtocolError::PayloadTooLarge {
            limit: MAX_BINARY_PAYLOAD_BYTES,
        });
    }
    Ok(decoded)
}
