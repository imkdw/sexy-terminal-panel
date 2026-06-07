#![allow(clippy::expect_used)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

mod panel_terminate_support;

use panel_terminate_support::{
    active_pane_key, assert_tmux_session_exists, kill_tmux_server, launch_panel, pane_keys,
    register_detached_terminal, set_pane_key, tmux_messages, wait_for_active_pane_key,
    wait_for_pane_title,
};

#[test]
fn panel_sidebar_click_focuses_visible_session_pane() {
    let fixture = ClickFixture::new("focus", 2);
    fixture.launch();
    wait_for_pane_title(&fixture.socket, fixture.terminal_id(1));
    wait_for_pane_title(&fixture.socket, fixture.terminal_id(2));
    wait_for_active_pane_key(&fixture.socket, fixture.terminal_id(1));

    fixture.panel_select("4");

    assert_eq!(active_pane_key(&fixture.socket), fixture.terminal_id(2));
    fixture.cleanup();
}

#[test]
fn panel_sidebar_click_opens_overflow_session_in_empty_right_pane() {
    let fixture = ClickFixture::new("overflow-empty", 10);
    fixture.launch();
    let empty_pane = wait_for_pane_title(&fixture.socket, fixture.terminal_id(2));
    set_pane_key(&fixture.socket, &empty_pane, "empty:2");
    wait_for_pane_title(&fixture.socket, fixture.terminal_id(9));

    fixture.panel_select("12");

    assert_eq!(active_pane_key(&fixture.socket), fixture.terminal_id(10));
    assert_eq!(
        wait_for_pane_title(&fixture.socket, fixture.terminal_id(10)),
        empty_pane
    );
    assert!(pane_keys(&fixture.socket).contains(&fixture.terminal_id(9).to_owned()));
    fixture.cleanup();
}

#[test]
fn panel_sidebar_click_opens_overflow_session_in_rightmost_pane() {
    let fixture = ClickFixture::new("overflow-rightmost", 10);
    fixture.launch();
    wait_for_pane_title(&fixture.socket, fixture.terminal_id(9));

    fixture.panel_select("12");

    assert_eq!(active_pane_key(&fixture.socket), fixture.terminal_id(10));
    assert!(pane_keys(&fixture.socket).contains(&fixture.terminal_id(10).to_owned()));
    assert!(!pane_keys(&fixture.socket).contains(&fixture.terminal_id(9).to_owned()));
    fixture.cleanup();
}

#[test]
fn panel_sidebar_click_on_header_is_noop() {
    let fixture = ClickFixture::new("invalid", 2);
    fixture.launch();
    let first_pane = wait_for_pane_title(&fixture.socket, fixture.terminal_id(1));
    fixture.select_pane(&first_pane);
    fixture.panel_select("1");

    assert_eq!(active_pane_key(&fixture.socket), fixture.terminal_id(1));
    assert!(tmux_messages(&fixture.socket).contains("No STP session for sidebar row"));
    assert_tmux_session_exists(&fixture.socket, &format!("stp-{}", fixture.terminal_id(1)));
    assert_tmux_session_exists(&fixture.socket, &format!("stp-{}", fixture.terminal_id(2)));
    fixture.cleanup();
}

#[test]
fn panel_sidebar_click_dead_rendered_row_does_not_select_next_session() {
    let fixture = ClickFixture::new("dead-row", 2);
    fixture.launch();
    wait_for_pane_title(&fixture.socket, fixture.terminal_id(1));
    wait_for_pane_title(&fixture.socket, fixture.terminal_id(2));
    wait_for_active_pane_key(&fixture.socket, fixture.terminal_id(1));

    fixture.kill_terminal_session(1);
    fixture.panel_select("3");

    assert_ne!(active_pane_key(&fixture.socket), fixture.terminal_id(2));
    assert!(tmux_messages(&fixture.socket).contains("STP session is no longer live"));
    assert_tmux_session_exists(&fixture.socket, &format!("stp-{}", fixture.terminal_id(2)));
    fixture.cleanup();
}

struct ClickFixture {
    temp: TempDir,
    registry: std::path::PathBuf,
    binary: std::path::PathBuf,
    socket: String,
    panel_socket: String,
    panel_session: String,
    terminals: Vec<String>,
}

impl ClickFixture {
    fn new(label: &str, terminal_count: usize) -> Self {
        let temp = TempDir::new().expect("temp dir");
        let registry = temp.path().join("registry.json");
        let socket = format!("stp-cli-click-{label}-{}", std::process::id());
        let panel_socket = format!("stp-cli-click-{label}-outer-{}", std::process::id());
        let panel_session = format!("stp-cli-click-{label}-wrapper");
        let terminals = (1..=terminal_count)
            .map(|index| format!("00000000-0000-0000-0000-0000000008{index:02}"))
            .collect::<Vec<_>>();
        let fixture = Self {
            temp,
            registry,
            binary: cargo_bin("stp"),
            socket,
            panel_socket,
            panel_session,
            terminals,
        };
        fixture.register_terminals();
        fixture
    }

    fn register_terminals(&self) {
        kill_tmux_server(&self.socket);
        kill_tmux_server(&self.panel_socket);
        for (index, terminal_id) in self.terminals.iter().enumerate() {
            let workspace = self.temp.path().join(format!("worktree-click-{index}"));
            std::fs::create_dir(&workspace).expect("workspace");
            register_detached_terminal(&self.registry, &workspace, &self.socket, terminal_id);
        }
    }

    fn launch(&self) {
        launch_panel(
            &self.panel_socket,
            &self.panel_session,
            &self.socket,
            &self.binary,
            &self.registry,
        );
    }

    fn panel_select(&self, mouse_line: &str) {
        Command::cargo_bin("stp")
            .expect("stp binary")
            .args([
                "panel-select",
                "--registry",
                self.registry.to_str().expect("registry path"),
                "--socket",
                &self.socket,
                "--mouse-line",
                mouse_line,
            ])
            .assert()
            .success();
    }

    fn select_pane(&self, pane_id: &str) {
        Command::new("tmux")
            .args(["-L", &self.socket, "select-pane", "-t", pane_id])
            .assert()
            .success();
    }

    fn kill_terminal_session(&self, one_based: usize) {
        Command::new("tmux")
            .args([
                "-L",
                &self.socket,
                "kill-session",
                "-t",
                &format!("stp-{}", self.terminal_id(one_based)),
            ])
            .assert()
            .success();
    }

    fn terminal_id(&self, one_based: usize) -> &str {
        &self.terminals[one_based.saturating_sub(1)]
    }

    fn cleanup(&self) {
        kill_tmux_server(&self.panel_socket);
        kill_tmux_server(&self.socket);
    }
}
