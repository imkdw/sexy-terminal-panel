use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("failed to encode protocol frame: {0}")]
    Encode(serde_json::Error),
    #[error("failed to decode protocol frame: {0}")]
    Decode(serde_json::Error),
    #[error("unsupported protocol version {version}")]
    UnsupportedVersion { version: u16 },
    #[error("malformed base64 payload: {source}")]
    MalformedBase64 { source: base64::DecodeError },
    #[error("protocol frame exceeds {limit} bytes")]
    FrameTooLarge { limit: usize },
    #[error("binary payload exceeds {limit} bytes")]
    PayloadTooLarge { limit: usize },
}
