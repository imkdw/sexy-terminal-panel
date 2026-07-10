#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(dead_code)]

use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

#[derive(Debug)]
pub struct AutoConnectFixture {
    _temp: TempDir,
    workspace: std::path::PathBuf,
    registry: std::path::PathBuf,
    binary: std::path::PathBuf,
    pub socket: String,
    panel_socket: String,
    panel_session: String,
}

impl AutoConnectFixture {
    pub fn new(label: &str) -> Self {
        let temp = TempDir::new().expect("temp dir");
        let workspace = temp.path().join(format!("worktree-auto-connect-{label}"));
        std::fs::create_dir(&workspace).expect("workspace");
        let socket = format!("stp-cli-auto-connect-{label}-test-{}", std::process::id());
        let panel_socket = format!("stp-cli-auto-connect-{label}-outer-{}", std::process::id());
        kill_tmux_server(&socket);
        kill_tmux_server(&panel_socket);
        Self {
            registry: temp.path().join("registry.json"),
            binary: cargo_bin("stp"),
            panel_session: format!("stp-cli-auto-connect-{label}-wrapper"),
            _temp: temp,
            workspace,
            socket,
            panel_socket,
        }
    }

    pub fn register(&self, terminal_id: &str) {
        let output = self
            .register_command(terminal_id)
            .output()
            .expect("run stp terminal");
        assert!(
            output.status.success(),
            "stp terminal failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub fn spawn_register(&self, terminal_id: &str) -> std::process::Child {
        self.register_command(terminal_id)
            .spawn()
            .expect("spawn stp terminal")
    }

    pub fn launch_panel(&self, layout: &str) {
        let command = format!(
            "STP_TMUX_SOCKET={} {} panel --registry {} --layout {}",
            shell_quote(&self.socket),
            shell_quote(&self.binary.display().to_string()),
            shell_quote(&self.registry.display().to_string()),
            shell_quote(layout),
        );
        let output = std::process::Command::new("tmux")
            .args([
                "-L",
                &self.panel_socket,
                "new-session",
                "-d",
                "-s",
                &self.panel_session,
                &command,
            ])
            .output()
            .expect("launch wrapped panel");
        assert!(
            output.status.success(),
            "tmux panel wrapper failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn register_command(&self, terminal_id: &str) -> std::process::Command {
        let mut command = std::process::Command::new(&self.binary);
        command.args([
            "terminal",
            "--workspace",
            self.workspace.to_str().expect("utf8 workspace"),
            "--window-id",
            "00000000-0000-0000-0000-000000000001",
            "--terminal-id",
            terminal_id,
            "--socket",
            &self.socket,
            "--registry",
            self.registry.to_str().expect("utf8 registry"),
            "--shell",
            "sh",
            "--detach",
        ]);
        command
    }
}

impl Drop for AutoConnectFixture {
    fn drop(&mut self) {
        kill_tmux_server(&self.panel_socket);
        kill_tmux_server(&self.socket);
    }
}

#[derive(Clone, Debug)]
pub struct PanelPane {
    pub id: String,
    pub key: String,
}

pub fn wait_for_layout_panes(socket: &str, expected_count: usize) -> Vec<PanelPane> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let panes = list_layout_panes(socket);
        if panes.len() == expected_count
            && panes.iter().any(|pane| pane.key == "stp-sidebar")
            && panes.iter().all(|pane| !pane.key.is_empty())
        {
            return panes;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {expected_count} layout panes; got {panes:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub fn wait_for_panel_pane_key(socket: &str, expected_key: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let panes = list_layout_panes(socket);
        if panes.iter().any(|pane| pane.key == expected_key) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for panel pane {expected_key}; got {panes:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub fn list_layout_panes(socket: &str) -> Vec<PanelPane> {
    let output = std::process::Command::new("tmux")
        .args([
            "-L",
            socket,
            "list-panes",
            "-t",
            "stp-panel",
            "-F",
            concat!("#{pane_id}\t#", "{@stp-pane-key}"),
        ])
        .output()
        .expect("layout panes");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_panel_pane)
        .collect()
}

pub fn capture_pane(socket: &str, pane_id: &str) -> String {
    tmux_output(socket, &["capture-pane", "-pt", pane_id, "-S", "-20"])
}

pub fn tmux_output(socket: &str, args: &[&str]) -> String {
    let output = std::process::Command::new("tmux")
        .arg("-L")
        .arg(socket)
        .args(args)
        .output()
        .expect("tmux command");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn wait_for_success(mut child: std::process::Child) {
    let status = child.wait().expect("wait for stp terminal");
    assert!(status.success(), "stp terminal exited with {status}");
}

fn parse_panel_pane(line: &str) -> Option<PanelPane> {
    let mut parts = line.split('\t');
    Some(PanelPane {
        id: parts.next()?.to_owned(),
        key: parts.next()?.to_owned(),
    })
}

fn kill_tmux_server(socket: &str) {
    drop(
        std::process::Command::new("tmux")
            .args(["-L", socket, "kill-server"])
            .output(),
    );
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
