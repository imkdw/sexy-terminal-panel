use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TmuxOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Error)]
pub enum TmuxError {
    #[error("failed to spawn tmux for socket {socket}: {source}")]
    Spawn {
        socket: String,
        source: std::io::Error,
    },
    #[error(
        "tmux command failed on socket {socket} with status {status} for `{command}`: {stderr}{stdout}"
    )]
    CommandFailed {
        socket: String,
        command: String,
        status: i32,
        stdout: String,
        stderr: String,
    },
}

impl TmuxError {
    pub fn is_missing_session(&self) -> bool {
        match self {
            Self::Spawn { .. } => false,
            Self::CommandFailed { stdout, stderr, .. } => {
                stdout.contains("can't find session")
                    || stderr.contains("can't find session")
                    || stdout.contains("can't find window")
                    || stderr.contains("can't find window")
                    || stdout.contains("no server running")
                    || stderr.contains("no server running")
                    || stdout.contains("error connecting")
                    || stderr.contains("error connecting")
            }
        }
    }
}
