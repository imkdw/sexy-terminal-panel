#![allow(clippy::expect_used)]

use std::path::PathBuf;

use stp_core::ids::{TerminalId, WindowId, WorkspaceId};
use stp_core::registry::{ManagedTerminal, TerminalBackend, TerminalStatus};

pub(super) fn terminal(id: &str, workspace: &str, branch: &str) -> ManagedTerminal {
    ManagedTerminal {
        terminal_id: TerminalId::parse(id).expect("terminal id"),
        workspace_id: WorkspaceId::new("workspace".to_owned()),
        window_id: WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id"),
        workspace_path: PathBuf::from(workspace),
        repo_root: PathBuf::from(workspace),
        branch_name: Some(branch.to_owned()),
        backend: TerminalBackend::legacy_tmux(
            "stp-test-socket".to_owned(),
            "stp-test-session".to_owned(),
            "0".to_owned(),
        ),
        tmux_socket: "stp-test-socket".to_owned(),
        tmux_session: "stp-test-session".to_owned(),
        tmux_window: "0".to_owned(),
        created_at: 0,
        last_seen_at: 0,
        status: TerminalStatus::Live,
    }
}
