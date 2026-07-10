pub use crate::binding::BindingCommand;
pub use crate::error::{TmuxError, TmuxOutput};
pub use crate::pane::PaneInfo;
pub use crate::session::{TmuxWaitForLock, TmuxWindowSession, TmuxWindowSize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tmux {
    pub(crate) socket_name: String,
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

    pub fn split_window_with_id(
        &self,
        target: &str,
        shell_command: &str,
    ) -> Result<String, TmuxError> {
        self.split_window_with_id_options(target, None, false, shell_command)
    }

    pub fn split_window_percent_with_id(
        &self,
        target: &str,
        percent: usize,
        shell_command: &str,
    ) -> Result<String, TmuxError> {
        self.split_window_with_id_options(target, Some(percent), false, shell_command)
    }

    pub fn split_window_right_with_id(
        &self,
        target: &str,
        shell_command: &str,
    ) -> Result<String, TmuxError> {
        self.split_window_with_id_options(target, None, true, shell_command)
    }

    pub fn split_window_right_percent_with_id(
        &self,
        target: &str,
        percent: usize,
        shell_command: &str,
    ) -> Result<String, TmuxError> {
        self.split_window_with_id_options(target, Some(percent), true, shell_command)
    }

    fn split_window_with_id_options(
        &self,
        target: &str,
        percent: Option<usize>,
        horizontal: bool,
        shell_command: &str,
    ) -> Result<String, TmuxError> {
        let size = percent.map(|value| value.to_string());
        let mut args = vec!["split-window", "-d"];
        if horizontal {
            args.push("-h");
        }
        if let Some(size) = size.as_deref() {
            args.extend(["-p", size]);
        }
        args.extend(["-P", "-F", "#{pane_id}", "-t", target, shell_command]);
        let output = self.run_args(&args)?;
        Ok(output.stdout.trim().to_owned())
    }

    pub fn split_window_left(
        &self,
        target: &str,
        width: usize,
        shell_command: &str,
    ) -> Result<(), TmuxError> {
        let size = width.to_string();
        self.run([
            "split-window",
            "-d",
            "-h",
            "-b",
            "-l",
            size.as_str(),
            "-t",
            target,
            shell_command,
        ])
        .map(drop)
    }

    pub fn select_tiled_layout(&self, target: &str) -> Result<(), TmuxError> {
        self.run(["select-layout", "-t", target, "tiled"]).map(drop)
    }

    pub fn bind_key(&self, key: &str, command: &str) -> Result<(), TmuxError> {
        self.run(["bind-key", key, command]).map(drop)
    }

    pub fn bind_key_command(
        &self,
        key: &str,
        command: &BindingCommand<'_>,
    ) -> Result<(), TmuxError> {
        let mut tmux_args = Vec::with_capacity(command.arguments.len().saturating_add(3));
        tmux_args.push("bind-key");
        tmux_args.push(key);
        tmux_args.push(command.command);
        tmux_args.extend(command.arguments.iter().copied());
        self.run_args(&tmux_args).map(drop)
    }

    pub fn bind_key_in_table(
        &self,
        table: &str,
        key: &str,
        command: &BindingCommand<'_>,
    ) -> Result<(), TmuxError> {
        let mut tmux_args = Vec::with_capacity(command.arguments.len().saturating_add(5));
        tmux_args.push("bind-key");
        tmux_args.push("-T");
        tmux_args.push(table);
        tmux_args.push(key);
        tmux_args.push(command.command);
        tmux_args.extend(command.arguments.iter().copied());
        self.run_args(&tmux_args).map(drop)
    }

    pub fn set_pane_title(&self, target: &str, title: &str) -> Result<(), TmuxError> {
        self.run(["select-pane", "-t", target, "-T", title])
            .map(drop)
    }

    pub fn set_pane_option(&self, target: &str, name: &str, value: &str) -> Result<(), TmuxError> {
        self.run(["set-option", "-p", "-t", target, name, value])
            .map(drop)
    }

    pub fn select_pane(&self, target: &str) -> Result<(), TmuxError> {
        self.run(["select-pane", "-t", target]).map(drop)
    }

    pub fn display_message(&self, target: &str, message: &str) -> Result<(), TmuxError> {
        self.run(["display-message", "-t", target, message])
            .map(drop)
    }

    pub fn resize_pane_width(&self, target: &str, width: usize) -> Result<(), TmuxError> {
        let size = width.to_string();
        self.run(["resize-pane", "-t", target, "-x", size.as_str()])
            .map(drop)
    }

    pub fn list_pane_ids(&self, target: &str) -> Result<Vec<String>, TmuxError> {
        let output = self.run(["list-panes", "-t", target, "-F", "#{pane_id}"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    pub fn list_panes_with_titles(&self, target: &str) -> Result<Vec<PaneInfo>, TmuxError> {
        let output = self.run([
            "list-panes",
            "-t",
            target,
            "-F",
            "#{pane_id}\t#{pane_title}\t#{@stp-pane-key}",
        ])?;
        Ok(output
            .stdout
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let pane_id = parts.next()?;
                let title = parts.next()?;
                let pane_key = parts.next().map_or("", |value| value);
                Some(PaneInfo {
                    pane_id: pane_id.to_owned(),
                    title: title.to_owned(),
                    pane_key: pane_key.to_owned(),
                })
            })
            .collect())
    }

    pub fn respawn_pane(&self, target: &str, shell_command: &str) -> Result<(), TmuxError> {
        self.run(["respawn-pane", "-k", "-t", target, shell_command])
            .map(drop)
    }

    pub fn set_option(&self, target: &str, name: &str, value: &str) -> Result<(), TmuxError> {
        self.run(["set-option", "-t", target, name, value])
            .map(drop)
    }

    pub fn set_window_option(
        &self,
        target: &str,
        name: &str,
        value: &str,
    ) -> Result<(), TmuxError> {
        self.run(["set-window-option", "-t", target, name, value])
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
}
