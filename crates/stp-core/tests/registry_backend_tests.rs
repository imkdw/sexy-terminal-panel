#![allow(clippy::expect_used)]

use std::fs;

use stp_core::ids::{TerminalId, WindowId};
use stp_core::registry::{
    ManagedTerminal, Registry, RegistryStore, SessionEndpoint, TerminalBackend,
};
use tempfile::TempDir;

#[test]
fn registry_deserializes_legacy_tmux_entries_as_legacy_backend() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-a");
    fs::create_dir(&workspace).expect("workspace");
    let path = temp.path().join("registry.json");
    fs::write(&path, legacy_registry_json(&workspace, temp.path())).expect("write legacy registry");
    let store = RegistryStore::new(path);

    let loaded = store.load().expect("load legacy registry");
    let terminal = loaded
        .terminal(&TerminalId::parse("00000000-0000-0000-0000-000000000401").expect("id"))
        .expect("legacy terminal");

    assert_eq!(
        terminal.backend,
        TerminalBackend::LegacyTmux {
            socket: "stp-managed".to_owned(),
            session: "stp-000000000401".to_owned(),
            window: "0".to_owned(),
        }
    );
}

#[test]
fn registry_serializes_pty_entries_without_tmux_fields() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-a");
    fs::create_dir(&workspace).expect("workspace");
    let store = RegistryStore::new(temp.path().join("registry.json"));
    let terminal = ManagedTerminal::new_pty(
        TerminalId::parse("00000000-0000-0000-0000-000000000402").expect("terminal id"),
        WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id"),
        &workspace,
        SessionEndpoint::unix_socket(temp.path().join("stp.sock")),
    )
    .expect("pty terminal");
    let mut registry = Registry::default();
    registry.upsert(terminal);

    store.save(&registry).expect("save registry");

    let encoded = fs::read_to_string(store.path()).expect("read registry");
    assert!(encoded.contains(r#""kind": "pty""#));
    assert!(encoded.contains(r#""socket_path""#));
    assert!(!encoded.contains("tmux_socket"));
    assert!(!encoded.contains("tmux_session"));
    assert!(!encoded.contains("tmux_window"));
}

#[test]
fn registry_rejects_missing_pty_endpoint_for_live_pty_terminal() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-a");
    fs::create_dir(&workspace).expect("workspace");
    let path = temp.path().join("registry.json");
    fs::write(
        &path,
        missing_pty_endpoint_registry_json(&workspace, temp.path()),
    )
    .expect("write bad registry");
    let store = RegistryStore::new(path);

    let err = store.load().expect_err("missing pty endpoint should fail");
    assert!(err.to_string().contains("malformed registry"));
}

fn legacy_registry_json(workspace: &std::path::Path, root: &std::path::Path) -> String {
    format!(
        r#"{{
  "terminals": [
    {{
      "terminal_id": "00000000-0000-0000-0000-000000000401",
      "workspace_id": "workspace-a",
      "window_id": "00000000-0000-0000-0000-000000000001",
      "workspace_path": "{}",
      "repo_root": "{}",
      "branch_name": null,
      "tmux_socket": "stp-managed",
      "tmux_session": "stp-000000000401",
      "tmux_window": "0",
      "created_at": 1,
      "last_seen_at": 2,
      "status": "live"
    }}
  ]
}}"#,
        workspace.display(),
        root.display(),
    )
}

fn missing_pty_endpoint_registry_json(
    workspace: &std::path::Path,
    root: &std::path::Path,
) -> String {
    format!(
        r#"{{
  "terminals": [
    {{
      "terminal_id": "00000000-0000-0000-0000-000000000403",
      "workspace_id": "workspace-a",
      "window_id": "00000000-0000-0000-0000-000000000001",
      "workspace_path": "{}",
      "repo_root": "{}",
      "branch_name": null,
      "backend": {{ "kind": "pty" }},
      "created_at": 1,
      "last_seen_at": 2,
      "status": "live"
    }}
  ]
}}"#,
        workspace.display(),
        root.display(),
    )
}
