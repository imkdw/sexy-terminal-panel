use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tmux {
    socket_name: String,
}

impl Tmux {
    pub fn new(socket_name: &str) -> Self {
        Self {
            socket_name: socket_name.to_owned(),
        }
    }

    pub fn new_session(&self, session_name: &str, shell_command: &str) -> Result<(), TmuxError> {
        self.run(["new-session", "-d", "-s", session_name, shell_command])
            .map(drop)
    }

    pub fn new_session_with_window(
        &self,
        session_name: &str,
        window_name: &str,
        shell_command: &str,
    ) -> Result<(), TmuxError> {
        self.run([
            "new-session",
            "-d",
            "-s",
            session_name,
            "-n",
            window_name,
            shell_command,
        ])
        .map(drop)
    }

    pub fn attach_session(&self, session_name: &str) -> Result<(), TmuxError> {
        self.run_attached(["attach-session", "-t", session_name])
    }

    pub fn kill_session_if_exists(&self, session_name: &str) -> Result<(), TmuxError> {
        match self.run(["kill-session", "-t", session_name]) {
            Ok(_output) => Ok(()),
            Err(error) if error.is_missing_session() => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn split_window(&self, target: &str, shell_command: &str) -> Result<(), TmuxError> {
        self.run(["split-window", "-d", "-t", target, shell_command])
            .map(drop)
    }

    pub fn select_tiled_layout(&self, target: &str) -> Result<(), TmuxError> {
        self.run(["select-layout", "-t", target, "tiled"]).map(drop)
    }

    pub fn set_option(&self, target: &str, name: &str, value: &str) -> Result<(), TmuxError> {
        self.run(["set-option", "-t", target, name, value])
            .map(drop)
    }

    pub fn send_keys(&self, target: &str, text: &str, enter: bool) -> Result<(), TmuxError> {
        if enter {
            self.run(["send-keys", "-t", target, text, "Enter"])
                .map(drop)
        } else {
            self.run(["send-keys", "-t", target, text]).map(drop)
        }
    }

    pub fn capture_pane(&self, target: &str, lines: usize) -> Result<String, TmuxError> {
        let start = format!("-{lines}");
        let output = self.run(["capture-pane", "-pt", target, "-S", start.as_str()])?;
        Ok(output.stdout)
    }

    pub fn list_sessions(&self) -> Result<Vec<String>, TmuxError> {
        let output = self.run(["list-sessions", "-F", "#{session_name}"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    pub fn kill_server(&self) -> Result<(), TmuxError> {
        self.run(["kill-server"]).map(drop)
    }

    fn run<const N: usize>(&self, args: [&str; N]) -> Result<TmuxOutput, TmuxError> {
        let output = Command::new("tmux")
            .arg("-L")
            .arg(&self.socket_name)
            .args(args)
            .output()
            .map_err(|source| TmuxError::Spawn {
                socket: self.socket_name.clone(),
                source,
            })?;
        let status = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let command = args.join(" ");
        if output.status.success() {
            Ok(TmuxOutput {
                status,
                stdout,
                stderr,
            })
        } else {
            Err(TmuxError::CommandFailed {
                socket: self.socket_name.clone(),
                command,
                status,
                stdout,
                stderr,
            })
        }
    }

    fn run_attached<const N: usize>(&self, args: [&str; N]) -> Result<(), TmuxError> {
        let status = Command::new("tmux")
            .arg("-L")
            .arg(&self.socket_name)
            .args(args)
            .env_remove("TMUX")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|source| TmuxError::Spawn {
                socket: self.socket_name.clone(),
                source,
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(TmuxError::CommandFailed {
                socket: self.socket_name.clone(),
                command: args.join(" "),
                status: status.code().unwrap_or(-1),
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }
}

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
    fn is_missing_session(&self) -> bool {
        match self {
            Self::Spawn { .. } => false,
            Self::CommandFailed { stdout, stderr, .. } => {
                stdout.contains("can't find session")
                    || stderr.contains("can't find session")
                    || stdout.contains("no server running")
                    || stderr.contains("no server running")
            }
        }
    }
}
