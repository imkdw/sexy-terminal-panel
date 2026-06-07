use std::collections::{BTreeMap, BTreeSet};

use stp_core::ids::TerminalId;
use stp_core::registry::{ManagedTerminal, Registry, RegistryError, RegistryStore, TerminalStatus};
use stp_tmux::adapter::Tmux;

pub fn load_without_zombie_sessions(store: &RegistryStore) -> Result<Registry, RegistryError> {
    let registry = store.load()?;
    let initially_missing = missing_live_terminals(&registry);
    if initially_missing.is_empty() {
        return Ok(registry);
    }

    let mut latest = store.load()?;
    let still_missing = missing_live_terminals(&latest);
    let confirmed_missing = initially_missing
        .intersection(&still_missing)
        .cloned()
        .collect::<BTreeSet<_>>();
    if remove_missing_live_terminals(&mut latest, &confirmed_missing) > 0 {
        store.save(&latest)?;
    }
    Ok(latest)
}

pub fn terminal_session_is_known_missing(terminal: &ManagedTerminal) -> bool {
    terminal.status == TerminalStatus::Live
        && list_sessions_if_available(&terminal.tmux_socket)
            .is_known_missing(&terminal.tmux_session)
}

pub fn mark_missing_live_sessions_stale(registry: &mut Registry) -> bool {
    let mut sessions_by_socket: BTreeMap<String, SocketSessions> = BTreeMap::new();
    let mut changed = false;

    for terminal in registry
        .terminals
        .iter_mut()
        .filter(|terminal| terminal.status == TerminalStatus::Live)
    {
        let sessions = sessions_by_socket
            .entry(terminal.tmux_socket.clone())
            .or_insert_with(|| list_sessions_if_available(&terminal.tmux_socket));
        if sessions.is_known_missing(&terminal.tmux_session) {
            terminal.status = TerminalStatus::Stale;
            changed = true;
        }
    }

    changed
}

pub fn remove_zombie_sessions(registry: &mut Registry) -> usize {
    let before = registry.terminals.len();
    mark_missing_live_sessions_stale(registry);
    registry.remove_stale();
    before.saturating_sub(registry.terminals.len())
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MissingLiveTerminal {
    terminal_id: TerminalId,
    tmux_socket: String,
    tmux_session: String,
}

impl MissingLiveTerminal {
    fn from_terminal(terminal: &ManagedTerminal) -> Self {
        Self {
            terminal_id: terminal.terminal_id.clone(),
            tmux_socket: terminal.tmux_socket.clone(),
            tmux_session: terminal.tmux_session.clone(),
        }
    }
}

fn missing_live_terminals(registry: &Registry) -> BTreeSet<MissingLiveTerminal> {
    let mut sessions_by_socket: BTreeMap<String, SocketSessions> = BTreeMap::new();
    let mut terminals = BTreeSet::new();

    for terminal in registry
        .terminals
        .iter()
        .filter(|terminal| terminal.status == TerminalStatus::Live)
    {
        let sessions = sessions_by_socket
            .entry(terminal.tmux_socket.clone())
            .or_insert_with(|| list_sessions_if_available(&terminal.tmux_socket));
        if sessions.is_known_missing(&terminal.tmux_session) {
            terminals.insert(MissingLiveTerminal::from_terminal(terminal));
        }
    }

    terminals
}

fn remove_missing_live_terminals(
    registry: &mut Registry,
    terminals: &BTreeSet<MissingLiveTerminal>,
) -> usize {
    let before = registry.terminals.len();
    registry.terminals.retain(|terminal| {
        terminal.status != TerminalStatus::Live
            || !terminals.contains(&MissingLiveTerminal::from_terminal(terminal))
    });

    before.saturating_sub(registry.terminals.len())
}

#[derive(Debug)]
enum SocketSessions {
    Known(BTreeSet<String>),
    Unknown,
}

impl SocketSessions {
    fn is_known_missing(&self, session: &str) -> bool {
        match self {
            Self::Known(sessions) => !sessions.contains(session),
            Self::Unknown => false,
        }
    }
}

fn list_sessions_if_available(socket: &str) -> SocketSessions {
    match Tmux::new(socket).list_sessions() {
        Ok(sessions) => SocketSessions::Known(sessions.into_iter().collect()),
        Err(error) if error.is_missing_session() => SocketSessions::Known(BTreeSet::new()),
        Err(_error) => SocketSessions::Unknown,
    }
}
