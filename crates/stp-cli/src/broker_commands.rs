use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use stp_core::ids::{TerminalId, WindowId};
use stp_core::protocol::{ClientRequest, ServerEvent};
use stp_pty::{BrokerClient, BrokerConfig, BrokerStatus};

use crate::cli::{
    BrokerCaptureArgs, BrokerCommand, BrokerInputArgs, BrokerSocketArgs, BrokerSpawnArgs,
    BrokerStateArgs, BrokerSubcommand, BrokerTerminalArgs,
};
use crate::output::{stdout_line, stdout_text};
use crate::state::{selected_broker_socket_path, selected_registry_path};

const READY_TIMEOUT: Duration = Duration::from_secs(3);

pub fn broker(args: BrokerCommand) -> Result<()> {
    match args.command {
        BrokerSubcommand::Ensure(args) => ensure(args),
        BrokerSubcommand::Serve(args) => serve(args),
        BrokerSubcommand::Status(args) => status(&args),
        BrokerSubcommand::Stop(args) => stop(&args),
        BrokerSubcommand::Spawn(args) => spawn(args),
        BrokerSubcommand::Input(args) => input(&args),
        BrokerSubcommand::Capture(args) => capture(&args),
        BrokerSubcommand::Terminate(args) => terminate(&args),
        BrokerSubcommand::List(args) => list(&args),
    }
}

fn ensure(args: BrokerStateArgs) -> Result<()> {
    let executable = std::env::current_exe().context("failed to locate stp executable")?;
    let status = stp_pty::ensure_broker(&config(args), &executable)?;
    print_status(&status)
}

fn serve(args: BrokerStateArgs) -> Result<()> {
    stp_pty::serve(config(args)).map_err(Into::into)
}

fn status(args: &BrokerSocketArgs) -> Result<()> {
    let socket = selected_broker_socket_path(args.socket.clone());
    let status = stp_pty::broker_status(&socket)?;
    print_status(&status)
}

fn stop(args: &BrokerSocketArgs) -> Result<()> {
    let socket = selected_broker_socket_path(args.socket.clone());
    stp_pty::stop_broker(&socket)?;
    wait_for_cleanup(&socket)?;
    stdout_line("stopped broker")?;
    Ok(())
}

fn spawn(args: BrokerSpawnArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let window_id = WindowId::parse(&args.window_id)?;
    let socket = selected_broker_socket_path(args.state.socket);
    let mut client = BrokerClient::connect(&socket)?;
    match client.request(&ClientRequest::Spawn {
        terminal_id,
        window_id,
        workspace_path: args.workspace,
        shell: args.shell,
        command: None,
    })? {
        ServerEvent::Spawned {
            terminal_id,
            process_id,
        } => {
            stdout_line(&format!("spawned {terminal_id} pid {process_id:?}"))?;
            Ok(())
        }
        event => Err(anyhow!("unexpected broker event: {event:?}")),
    }
}

fn input(args: &BrokerInputArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let socket = selected_broker_socket_path(args.socket.socket.clone());
    let mut client = BrokerClient::connect(&socket)?;
    client.request(&ClientRequest::input_bytes(
        terminal_id.clone(),
        args.text.as_bytes(),
    ))?;
    stdout_line(&format!("sent input to {terminal_id}"))?;
    Ok(())
}

fn capture(args: &BrokerCaptureArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let socket = selected_broker_socket_path(args.socket.socket.clone());
    let mut client = BrokerClient::connect(&socket)?;
    match client.request(&ClientRequest::Capture {
        terminal_id,
        lines: Some(args.lines),
    })? {
        ServerEvent::Snapshot { text, .. } => {
            stdout_text(&text)?;
            Ok(())
        }
        event => Err(anyhow!("unexpected broker event: {event:?}")),
    }
}

fn terminate(args: &BrokerTerminalArgs) -> Result<()> {
    let terminal_id = TerminalId::parse(&args.terminal_id)?;
    let socket = selected_broker_socket_path(args.socket.socket.clone());
    let mut client = BrokerClient::connect(&socket)?;
    client.request(&ClientRequest::Terminate {
        terminal_id: terminal_id.clone(),
    })?;
    stdout_line(&format!("terminated {terminal_id}"))?;
    Ok(())
}

fn list(args: &BrokerSocketArgs) -> Result<()> {
    let socket = selected_broker_socket_path(args.socket.clone());
    let mut client = BrokerClient::connect(&socket)?;
    match client.request(&ClientRequest::List)? {
        ServerEvent::SessionList { sessions } => {
            for session in sessions {
                stdout_line(&format!("{} {}", session.terminal_id, session.status))?;
            }
            Ok(())
        }
        event => Err(anyhow!("unexpected broker event: {event:?}")),
    }
}

fn config(args: BrokerStateArgs) -> BrokerConfig {
    BrokerConfig::new(
        selected_registry_path(args.registry),
        selected_broker_socket_path(args.socket),
        READY_TIMEOUT,
    )
}

fn print_status(status: &BrokerStatus) -> Result<()> {
    stdout_line(&format!(
        "broker ready pid {} sessions {}",
        status.pid,
        status.sessions.len()
    ))?;
    Ok(())
}

fn wait_for_cleanup(socket: &Path) -> Result<()> {
    let deadline = Instant::now() + READY_TIMEOUT;
    let pid = stp_pty::pid_path_for_socket(socket);
    while Instant::now() < deadline {
        if !socket.exists() && !pid.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(anyhow!("broker files still exist after stop"))
}
