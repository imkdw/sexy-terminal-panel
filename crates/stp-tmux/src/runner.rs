use std::process::{Command, Stdio};

use crate::adapter::Tmux;
use crate::error::{TmuxError, TmuxOutput};

impl Tmux {
    pub(crate) fn run<const N: usize>(&self, args: [&str; N]) -> Result<TmuxOutput, TmuxError> {
        self.run_args(&args)
    }

    pub(crate) fn run_args(&self, args: &[&str]) -> Result<TmuxOutput, TmuxError> {
        let output = Command::new("tmux")
            .arg("-L")
            .arg(&self.socket_name)
            .args(args)
            .output()
            .map_err(|source| TmuxError::Spawn {
                socket: self.socket_name.clone(),
                source,
            })?;
        let status = output.status.code().map_or(-1, |code| code);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let command = args.join(" ");
        if output.status.success() {
            return Ok(TmuxOutput {
                status,
                stdout,
                stderr,
            });
        }
        Err(TmuxError::CommandFailed {
            socket: self.socket_name.clone(),
            command,
            status,
            stdout,
            stderr,
        })
    }

    pub(crate) fn run_attached<const N: usize>(&self, args: [&str; N]) -> Result<(), TmuxError> {
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
            return Ok(());
        }
        Err(TmuxError::CommandFailed {
            socket: self.socket_name.clone(),
            command: args.join(" "),
            status: status.code().map_or(-1, |code| code),
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}
