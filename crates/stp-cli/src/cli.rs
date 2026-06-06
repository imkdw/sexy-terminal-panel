use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "stp", about = "tmux-backed central terminal panel")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Terminal(TerminalArgs),
    Panel(PanelArgs),
    OpenCode(OpenCodeArgs),
    Terminate(TerminateArgs),
    QaSendFocused(SendFocusedArgs),
    QaCapture(CaptureArgs),
    Registry(RegistryCommand),
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
pub struct CommonStateArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
    #[arg(long, env = "STP_TMUX_SOCKET", default_value = "stp-managed")]
    pub socket: String,
}

#[derive(Debug, Args)]
pub struct TerminalArgs {
    #[command(flatten)]
    pub state: CommonStateArgs,
    #[arg(long)]
    pub workspace: PathBuf,
    #[arg(long)]
    pub window_id: String,
    #[arg(long)]
    pub terminal_id: String,
    #[arg(long)]
    pub shell: Option<String>,
    #[arg(long)]
    pub detach: bool,
}

#[derive(Debug, Args)]
pub struct PanelArgs {
    #[command(flatten)]
    pub state: CommonStateArgs,
    #[arg(long, default_value = "2x2")]
    pub layout: String,
    #[arg(long)]
    pub once: bool,
}

#[derive(Debug, Args)]
pub struct OpenCodeArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
    #[arg(long)]
    pub terminal_id: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub log: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct TerminateArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
    #[arg(long)]
    pub terminal_id: String,
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct SendFocusedArgs {
    #[command(flatten)]
    pub state: CommonStateArgs,
    #[arg(long)]
    pub terminal_id: String,
    #[arg(long)]
    pub text: String,
}

#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
    #[arg(long)]
    pub terminal_id: String,
    #[arg(long, default_value_t = 50)]
    pub lines: usize,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum RegistrySubcommand {
    RemoveStale(RemoveStaleArgs),
    CleanupZombies(CleanupZombiesArgs),
}

#[derive(Debug, Args)]
pub struct RegistryCommand {
    #[command(subcommand)]
    pub command: RegistrySubcommand,
}

#[derive(Debug, Args)]
pub struct RemoveStaleArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct CleanupZombiesArgs {
    #[arg(long, env = "STP_REGISTRY")]
    pub registry: Option<PathBuf>,
    #[arg(long)]
    pub yes: bool,
}
