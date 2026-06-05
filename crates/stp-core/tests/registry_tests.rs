#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::process::Command;

use stp_core::ids::{TerminalId, WindowId};
use stp_core::registry::{ManagedTerminal, Registry, RegistryStore, TerminalStatus};
use stp_core::workspace::discover_workspace;
use tempfile::TempDir;

#[test]
fn registry_round_trip_when_valid() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-a");
    fs::create_dir(&workspace).expect("workspace");
    let store = RegistryStore::new(temp.path().join("registry.json"));
    let terminal = ManagedTerminal::new(
        TerminalId::parse("00000000-0000-0000-0000-000000000101").expect("terminal id"),
        WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id"),
        &workspace,
        "stp-managed",
        "stp-000000000101",
    )
    .expect("managed terminal");

    let mut registry = Registry::default();
    registry.upsert(terminal);
    store.save(&registry).expect("save registry");

    let loaded = store.load().expect("load registry");
    let found = loaded
        .terminal(&TerminalId::parse("00000000-0000-0000-0000-000000000101").expect("id"))
        .expect("registered terminal");
    assert_eq!(found.status, TerminalStatus::Live);
    assert_eq!(
        found.workspace_path,
        workspace.canonicalize().expect("canonical path")
    );
}

#[test]
fn malformed_registry_when_invalid_json() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("registry.json");
    fs::write(&path, "{broken").expect("write bad json");
    let store = RegistryStore::new(path);

    let err = store.load().expect_err("malformed registry should fail");
    assert!(err.to_string().contains("malformed registry"));
}

#[test]
fn discover_worktrees_when_git_repo_has_two_worktrees() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, ["init"]);
    git(&repo, ["config", "user.email", "qa@example.test"]);
    git(&repo, ["config", "user.name", "QA"]);
    fs::write(repo.join("README.md"), "hello").expect("seed file");
    git(&repo, ["add", "README.md"]);
    git(&repo, ["commit", "-m", "seed"]);
    let wt_a = temp.path().join("worktree-a");
    let wt_b = temp.path().join("worktree-b");
    git(
        &repo,
        [
            "worktree",
            "add",
            wt_a.to_str().expect("utf8 a"),
            "-b",
            "feature/a",
        ],
    );
    git(
        &repo,
        [
            "worktree",
            "add",
            wt_b.to_str().expect("utf8 b"),
            "-b",
            "feature/b",
        ],
    );

    let meta_a = discover_workspace(&wt_a).expect("discover a");
    let meta_b = discover_workspace(&wt_b).expect("discover b");

    assert_eq!(meta_a.branch_name.as_deref(), Some("feature/a"));
    assert_eq!(meta_b.branch_name.as_deref(), Some("feature/b"));
    assert_eq!(
        meta_a.repo_root,
        repo.canonicalize().expect("repo canonical")
    );
}

#[test]
fn remove_stale_when_registry_contains_live_and_stale() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-a");
    fs::create_dir(&workspace).expect("workspace");
    let live_id = TerminalId::parse("00000000-0000-0000-0000-000000000301").expect("live id");
    let stale_id = TerminalId::parse("00000000-0000-0000-0000-000000000302").expect("stale id");
    let window_id = WindowId::parse("00000000-0000-0000-0000-000000000001").expect("window id");
    let mut registry = Registry::default();
    registry.upsert(
        ManagedTerminal::new(
            live_id.clone(),
            window_id.clone(),
            &workspace,
            "socket",
            "live",
        )
        .expect("live terminal"),
    );
    let mut stale =
        ManagedTerminal::new(stale_id.clone(), window_id, &workspace, "socket", "stale")
            .expect("stale terminal");
    stale.status = TerminalStatus::Stale;
    registry.upsert(stale);

    let removed = registry.remove_stale();

    assert_eq!(removed, 1);
    assert!(registry.terminal(&live_id).is_some());
    assert!(registry.terminal(&stale_id).is_none());
}

fn git<const N: usize>(cwd: &std::path::Path, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git command");
    assert!(output.status.success(), "git failed: {output:?}");
}
