use anyhow::{Context, bail};
use stp_core::registry::{ManagedTerminal, RegistryStore};
use stp_tmux::adapter::{PaneInfo, Tmux, TmuxError, TmuxWindowSession};

use super::Layout;
#[cfg(test)]
pub(super) use super::bindings::terminate_binding;
use super::bindings::{install_mouse_binding, install_quit_binding, install_terminate_binding};
use super::layout;
#[cfg(test)]
pub(super) use super::layout::{pane_commands, pane_titles};
use super::session_sidebar;
use super::terminal_size;
use super::tmux_grid;
use crate::session_cleanup::{load_without_zombie_sessions, terminal_session_is_known_missing};

const PANEL_SESSION: &str = "stp-panel";
const PANEL_WINDOW: &str = "panel";
const ACTIVE_BORDER_STYLE: &str = "fg=colour154";
const BORDER_STYLE: &str = "fg=colour244";
const PANE_BORDER_FORMAT: &str = " #{?pane_active,*, }#{pane_title} ";
const PANEL_CONNECT_LOCK: &str = "stp-panel-connect";

pub fn open(store: &RegistryStore, layout: Layout, panel_socket: &str) -> anyhow::Result<()> {
    let registry = load_without_zombie_sessions(store)?;
    let commands = layout::pane_commands(&registry, layout);
    let titles = layout::pane_titles(&registry, layout);
    let tmux = Tmux::new(panel_socket);
    tmux.kill_session_if_exists(PANEL_SESSION)?;
    create_panel_session(&tmux, &session_sidebar::command(&registry))?;
    configure_panel_options(&tmux)?;
    let sidebar_pane = tmux
        .list_pane_ids(PANEL_SESSION)?
        .first()
        .context("panel session has no sidebar pane")?
        .clone();
    set_pane_key(&tmux, &sidebar_pane, session_sidebar::TITLE)?;
    install_mouse_binding(&tmux, store.path(), panel_socket)?;
    install_quit_binding(&tmux)?;
    install_terminate_binding(&tmux, store.path(), panel_socket)?;
    let first_command = commands.first().context("panel layout has no panes")?;
    let first_content_pane = tmux.split_window_right_with_id(&sidebar_pane, first_command)?;
    tmux.resize_pane_width(&sidebar_pane, session_sidebar::WIDTH)?;
    let content_pane_ids =
        tmux_grid::split_content_grid(&tmux, &first_content_pane, &commands, layout)?;
    tmux.resize_pane_width(&sidebar_pane, session_sidebar::WIDTH)?;
    if content_pane_ids.len() != titles.len() {
        bail!(
            "panel pane count mismatch: expected {}, got {}",
            titles.len(),
            content_pane_ids.len()
        );
    }
    for (pane_id, title) in content_pane_ids.iter().zip(titles.iter()) {
        set_pane_key(&tmux, pane_id, title)?;
    }
    tmux.select_pane(&first_content_pane)?;
    tmux.attach_session(PANEL_SESSION)?;
    Ok(())
}

fn create_panel_session(tmux: &Tmux, sidebar_command: &str) -> anyhow::Result<()> {
    tmux.new_window_session(TmuxWindowSession {
        session_name: PANEL_SESSION,
        window_name: PANEL_WINDOW,
        shell_command: sidebar_command,
        size: terminal_size::current(),
    })?;
    Ok(())
}

pub fn select_from_sidebar(
    store: &RegistryStore,
    mouse_line: &str,
    panel_socket: &str,
) -> anyhow::Result<()> {
    let registry = store.load()?;
    let Some(terminal) = session_sidebar::terminal_for_mouse_line(&registry, mouse_line) else {
        Tmux::new(panel_socket).display_message(PANEL_SESSION, "No STP session for sidebar row")?;
        return Ok(());
    };
    let tmux = Tmux::new(panel_socket);
    if terminal_session_is_known_missing(&terminal) {
        tmux.display_message(PANEL_SESSION, "STP session is no longer live")?;
        return Ok(());
    }
    let panes = tmux.list_panes_with_titles(PANEL_SESSION)?;
    let terminal_id = terminal.terminal_id.to_string();
    if let Some(pane) = panes.iter().find(|pane| pane.pane_key == terminal_id) {
        tmux.select_pane(&pane.pane_id)?;
        return Ok(());
    }
    let target = panes
        .iter()
        .find(|pane| pane.pane_key.starts_with("empty:"))
        .or_else(|| {
            panes
                .iter()
                .rev()
                .find(|pane| pane.pane_key != session_sidebar::TITLE)
        })
        .context("panel has no content pane available for selection")?;
    tmux.respawn_pane(
        &target.pane_id,
        &session_sidebar::terminal_command(&terminal),
    )?;
    set_pane_key(&tmux, &target.pane_id, &terminal_id)?;
    tmux.select_pane(&target.pane_id)?;
    Ok(())
}

pub fn connect_registered_terminal(
    store: &RegistryStore,
    terminal: &ManagedTerminal,
    panel_socket: &str,
) -> anyhow::Result<bool> {
    let tmux = Tmux::new(panel_socket);
    let _lock = match tmux.wait_for_lock(PANEL_CONNECT_LOCK) {
        Ok(lock) => lock,
        Err(error) if error.is_missing_session() => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let panes = match tmux.list_panes_with_titles(PANEL_SESSION) {
        Ok(panes) => panes,
        Err(error) if error.is_missing_session() => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let terminal_id = terminal.terminal_id.to_string();
    if panes.iter().any(|pane| pane.pane_key == terminal_id) {
        return Ok(true);
    }
    let Some(target) = first_empty_pane(&panes) else {
        return Ok(false);
    };
    if let Err(error) = tmux.respawn_pane(
        &target.pane_id,
        &session_sidebar::terminal_command(terminal),
    ) {
        if error.is_missing_session() {
            return Ok(false);
        }
        return Err(error.into());
    }
    if let Err(error) = set_pane_key(&tmux, &target.pane_id, &terminal_id) {
        if error.is_missing_session() {
            return Ok(false);
        }
        return Err(error.into());
    }
    if !refresh_sidebar(&tmux, store)? {
        return Ok(false);
    }
    Ok(true)
}

pub(super) fn configure_panel_options(tmux: &Tmux) -> anyhow::Result<()> {
    tmux.set_option(PANEL_SESSION, "status", "off")?;
    tmux.set_option(PANEL_SESSION, "mouse", "on")?;
    tmux.set_window_option(PANEL_SESSION, "allow-rename", "off")?;
    tmux.set_window_option(PANEL_SESSION, "allow-set-title", "off")?;
    tmux.set_window_option(PANEL_SESSION, "automatic-rename", "off")?;
    tmux.set_window_option(PANEL_SESSION, "pane-border-status", "top")?;
    tmux.set_window_option(PANEL_SESSION, "pane-border-format", PANE_BORDER_FORMAT)?;
    tmux.set_window_option(PANEL_SESSION, "pane-border-style", BORDER_STYLE)?;
    tmux.set_window_option(
        PANEL_SESSION,
        "pane-active-border-style",
        ACTIVE_BORDER_STYLE,
    )?;
    Ok(())
}

fn refresh_sidebar(tmux: &Tmux, store: &RegistryStore) -> anyhow::Result<bool> {
    let panes = match tmux.list_panes_with_titles(PANEL_SESSION) {
        Ok(panes) => panes,
        Err(error) if error.is_missing_session() => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let Some(sidebar) = panes
        .iter()
        .find(|pane| pane.pane_key == session_sidebar::TITLE)
    else {
        return Ok(true);
    };
    let registry = store.load()?;
    if let Err(error) = tmux.respawn_pane(&sidebar.pane_id, &session_sidebar::command(&registry)) {
        if error.is_missing_session() {
            return Ok(false);
        }
        return Err(error.into());
    }
    if let Err(error) = set_pane_key(tmux, &sidebar.pane_id, session_sidebar::TITLE) {
        if error.is_missing_session() {
            return Ok(false);
        }
        return Err(error.into());
    }
    Ok(true)
}

fn set_pane_key(tmux: &Tmux, pane_id: &str, key: &str) -> Result<(), TmuxError> {
    let pane_title = pane_title_for_key(key);
    tmux.set_pane_title(pane_id, &pane_title)?;
    tmux.set_pane_option(pane_id, "@stp-pane-key", key)?;
    Ok(())
}

fn first_empty_pane(panes: &[PaneInfo]) -> Option<&PaneInfo> {
    panes
        .iter()
        .filter_map(|pane| empty_slot_number(&pane.pane_key).map(|slot| (slot, pane)))
        .min_by_key(|(slot, _pane)| *slot)
        .map(|(_slot, pane)| pane)
}

fn empty_slot_number(pane_key: &str) -> Option<usize> {
    pane_key.strip_prefix("empty:")?.parse().ok()
}

fn pane_title_for_key(key: &str) -> String {
    if let Some(slot) = key.strip_prefix("empty:") {
        return format!("slot {slot} empty");
    }
    let short_id = key.chars().take(8).collect::<String>();
    format!("STP {short_id}")
}
