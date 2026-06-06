use std::collections::{BTreeMap, BTreeSet};

use stp_core::registry::{Registry, TerminalStatus};
use stp_tmux::adapter::Tmux;

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
