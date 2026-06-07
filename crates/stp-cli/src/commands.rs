use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, anyhow, bail};
use stp_core::ids::{TerminalId, WindowId};
use stp_core::registry::{ManagedTerminal, RegistryStore, TerminalStatus};
use stp_tmux::adapter::Tmux;

use crate::cli::{
    CaptureArgs, CleanupZombiesArgs, DetachArgs, DoctorArgs, OpenCursorArgs, RemoveStaleArgs,
    TerminalArgs, TerminateArgs,
};
use crate::output::{stdout_line, stdout_text};
use crate::session_cleanup::remove_zombie_sessions;
use crate::state::selected_registry_path;

pub fn terminal(args: TerminalArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let window_id = WindowId::parse(&args.window_id)?;
    let session_name = format!("stp-{terminal_id}");
    let store = RegistryStore::new(selected_registry_path(args.state.registry));
    let mut registry = store.load()?;
    let terminal = ManagedTerminal::new(
        terminal_id,
        window_id,
        &args.workspace,
        &args.state.socket,
        &session_name,
    )?;
    registry.upsert(terminal);
    store.save(&registry)?;

    let tmux = Tmux::new(&args.state.socket);
    ensure_session(&tmux, &session_name, &args.workspace, args.shell.as_deref())?;
    if args.detach {
        stdout_line(&format!("registered {session_name}"))?;
        return Ok(());
    }
    tmux.attach_session(&session_name)?;
    Ok(())
}

pub fn send_focused(args: crate::cli::SendFocusedArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let store = RegistryStore::new(selected_registry_path(args.state.registry));
    let registry = store.load()?;
    let terminal = registry
        .terminal(&terminal_id)
        .ok_or_else(|| anyhow!("terminal not found: {terminal_id}"))?;
    let tmux = Tmux::new(&terminal.tmux_socket);
    tmux.send_keys(&terminal.tmux_session, &args.text, true)?;
    stdout_line(&format!("sent input to {terminal_id}"))?;
    Ok(())
}

pub fn capture(args: CaptureArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let store = RegistryStore::new(selected_registry_path(args.registry));
    let registry = store.load()?;
    let terminal = registry
        .terminal(&terminal_id)
        .ok_or_else(|| anyhow!("terminal not found: {terminal_id}"))?;
    let tmux = Tmux::new(&terminal.tmux_socket);
    let capture = tmux.capture_pane(&terminal.tmux_session, args.lines)?;
    stdout_text(&capture)?;
    Ok(())
}

pub fn open_cursor(args: OpenCursorArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let store = RegistryStore::new(selected_registry_path(args.registry));
    let registry = store.load()?;
    let terminal = registry
        .terminal(&terminal_id)
        .ok_or_else(|| anyhow!("terminal not found: {terminal_id}"))?;
    let command = format!(
        "cursor --new-window {}",
        shell_quote(&terminal.workspace_path)
    );
    if args.dry_run || std::env::var("STP_OPEN_CURSOR_DRY_RUN").is_ok() {
        write_optional_log(args.log.as_deref(), &command)?;
        stdout_line(&format!("fallback command: {command}"))?;
        return Ok(());
    }
    let status = ProcessCommand::new("cursor")
        .arg("--new-window")
        .arg(&terminal.workspace_path)
        .status()
        .context("cursor CLI not found")?;
    if !status.success() {
        bail!(
            "cursor CLI failed for {}",
            terminal.workspace_path.display()
        );
    }
    Ok(())
}

pub fn terminate(args: TerminateArgs) -> Result<()> {
    if !args.yes {
        bail!("refusing to terminate without --yes");
    }
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let store = RegistryStore::new(selected_registry_path(args.registry));
    let mut registry = store.load()?;
    let index = registry
        .terminals
        .iter()
        .position(|terminal| terminal.terminal_id == terminal_id)
        .ok_or_else(|| anyhow!("terminal not found: {terminal_id}"))?;
    let terminal = registry.terminals[index].clone();
    let tmux = Tmux::new(&terminal.tmux_socket);
    let session_existed = tmux.list_sessions().is_ok_and(|sessions| {
        sessions
            .iter()
            .any(|session| session == &terminal.tmux_session)
    });

    tmux.kill_session_if_exists(&terminal.tmux_session)?;
    registry.terminals[index].status = TerminalStatus::Exited;
    store.save(&registry)?;

    if session_existed {
        stdout_line(&format!("terminated {terminal_id}"))?;
    } else {
        stdout_line(&format!("already exited {terminal_id}"))?;
    }
    Ok(())
}

pub fn detach(args: DetachArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let store = RegistryStore::new(selected_registry_path(args.registry));
    let mut registry = store.load()?;
    let terminal = registry
        .terminals
        .iter_mut()
        .find(|terminal| terminal.terminal_id == terminal_id)
        .ok_or_else(|| anyhow!("terminal not found: {terminal_id}"))?;
    terminal.status = TerminalStatus::Detached;
    store.save(&registry)?;
    stdout_line(&format!("detached {terminal_id}"))?;
    Ok(())
}

pub fn remove_stale(args: RemoveStaleArgs) -> Result<()> {
    if !args.yes {
        bail!("refusing to remove stale entries without --yes");
    }
    let store = RegistryStore::new(selected_registry_path(args.registry));
    let mut registry = store.load()?;
    let removed = registry.remove_stale();
    store.save(&registry)?;
    stdout_line(&format!("removed stale entries: {removed}"))?;
    Ok(())
}

pub fn cleanup_zombies(args: CleanupZombiesArgs) -> Result<()> {
    if !args.yes {
        bail!("refusing to cleanup zombie entries without --yes");
    }
    let store = RegistryStore::new(selected_registry_path(args.registry));
    let mut registry = store.load()?;
    let removed = remove_zombie_sessions(&mut registry);
    store.save(&registry)?;
    stdout_line(&format!("removed zombie entries: {removed}"))?;
    Ok(())
}

pub fn doctor(args: DoctorArgs) -> Result<()> {
    check_command("tmux", &["-V"], "tmux not found")?;
    check_command("cursor", &["--version"], "cursor CLI not found")?;
    let path = selected_registry_path(args.registry);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("registry directory is inaccessible")?;
    }
    stdout_line(&format!("registry: {}", path.display()))?;
    stdout_line("doctor ok")?;
    Ok(())
}

fn ensure_session(
    tmux: &Tmux,
    session_name: &str,
    workspace: &Path,
    shell: Option<&str>,
) -> Result<()> {
    let exists = tmux
        .list_sessions()
        .is_ok_and(|sessions| sessions.iter().any(|session| session == session_name));
    if exists {
        return Ok(());
    }
    let shell = shell
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "sh".to_owned());
    let command = format!(
        "cd {} && exec {}",
        shell_quote(workspace),
        shell_quote(Path::new(&shell))
    );
    tmux.new_session(session_name, &command)?;
    Ok(())
}

fn check_command(name: &str, args: &[&str], message: &str) -> Result<()> {
    let status = ProcessCommand::new(name)
        .args(args)
        .status()
        .with_context(|| message.to_owned())?;
    if status.success() {
        Ok(())
    } else {
        bail!("{message}")
    }
}

fn write_optional_log(path: Option<&Path>, line: &str) -> Result<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{line}\n"))?;
    }
    Ok(())
}

fn shell_quote(path: &Path) -> String {
    let raw = path.display().to_string();
    format!("'{}'", raw.replace('\'', "'\\''"))
}
