use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod output;
mod panel;
mod session_cleanup;
mod state;

use cli::{Cli, Command, RegistrySubcommand};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Terminal(args) => commands::terminal(args),
        Command::Panel(args) => {
            let store = stp_core::registry::RegistryStore::new(state::selected_registry_path(
                args.state.registry,
            ));
            let layout = panel::Layout::parse(&args.layout);
            if args.once {
                let registry = session_cleanup::load_without_zombie_sessions(&store)?;
                let rendered = panel::render_once(&registry, layout)?;
                output::stdout_text(&rendered)?;
                Ok(())
            } else {
                panel::run_interactive(&store, layout, &args.state.socket)
            }
        }
        Command::PanelSelect(args) => {
            let store = stp_core::registry::RegistryStore::new(state::selected_registry_path(
                args.state.registry,
            ));
            panel::select_from_sidebar(&store, &args.mouse_line, &args.state.socket)
        }
        Command::OpenCursor(args) => commands::open_cursor(args),
        Command::Terminate(args) => commands::terminate(args),
        Command::Detach(args) => commands::detach(args),
        Command::QaSendFocused(args) => commands::send_focused(args),
        Command::QaCapture(args) => commands::capture(args),
        Command::Registry(args) => match args.command {
            RegistrySubcommand::RemoveStale(remove_args) => commands::remove_stale(remove_args),
            RegistrySubcommand::CleanupZombies(cleanup_args) => {
                commands::cleanup_zombies(cleanup_args)
            }
        },
        Command::Doctor(args) => commands::doctor(args),
    }
}
