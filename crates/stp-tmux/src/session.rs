use crate::adapter::Tmux;
use crate::error::TmuxError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TmuxWindowSize {
    cols: usize,
    rows: usize,
}

impl TmuxWindowSize {
    pub const fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }

    pub const fn cols(self) -> usize {
        self.cols
    }

    pub const fn rows(self) -> usize {
        self.rows
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TmuxWindowSession<'a> {
    pub session_name: &'a str,
    pub window_name: &'a str,
    pub shell_command: &'a str,
    pub size: Option<TmuxWindowSize>,
}

#[derive(Debug)]
pub struct TmuxWaitForLock<'a> {
    tmux: &'a Tmux,
    name: String,
}

impl Tmux {
    pub fn new_window_session(&self, spec: TmuxWindowSession<'_>) -> Result<(), TmuxError> {
        let cols = spec.size.map(|size| size.cols().to_string());
        let rows = spec.size.map(|size| size.rows().to_string());
        let mut args = vec!["new-session", "-d"];
        if let (Some(cols), Some(rows)) = (cols.as_deref(), rows.as_deref()) {
            args.extend(["-x", cols, "-y", rows]);
        }
        args.extend([
            "-s",
            spec.session_name,
            "-n",
            spec.window_name,
            spec.shell_command,
        ]);
        self.run_args(&args).map(drop)
    }

    pub fn wait_for_lock(&self, name: &str) -> Result<TmuxWaitForLock<'_>, TmuxError> {
        self.run(["wait-for", "-L", name])?;
        Ok(TmuxWaitForLock {
            tmux: self,
            name: name.to_owned(),
        })
    }
}

impl Drop for TmuxWaitForLock<'_> {
    fn drop(&mut self) {
        drop(self.tmux.run_args(&["wait-for", "-U", &self.name]));
    }
}
