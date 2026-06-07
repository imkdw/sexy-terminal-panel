use anyhow::{Context, bail};
use stp_core::registry::RegistryStore;
use stp_tmux::adapter::Tmux;

use super::Layout;
#[cfg(test)]
pub(super) use super::bindings::terminate_binding;
use super::bindings::{install_mouse_binding, install_quit_binding, install_terminate_binding};
use super::layout;
#[cfg(test)]
pub(super) use super::layout::{pane_commands, pane_titles};
use super::session_sidebar;
use crate::session_cleanup::mark_missing_live_sessions_stale;

const PANEL_SESSION: &str = "stp-panel";
const PANEL_WINDOW: &str = "panel";

pub fn open(store: &RegistryStore, layout: Layout, panel_socket: &str) -> anyhow::Result<()> {
    let mut registry = store.load()?;
    if mark_missing_live_sessions_stale(&mut registry) {
        store.save(&registry)?;
    }
    let commands = layout::pane_commands(&registry, layout);
    let titles = layout::pane_titles(&registry, layout);
    let tmux = Tmux::new(panel_socket);
    tmux.kill_session_if_exists(PANEL_SESSION)?;
    tmux.new_session_with_window(
        PANEL_SESSION,
        PANEL_WINDOW,
        &session_sidebar::command(&registry),
    )?;
    tmux.set_option(PANEL_SESSION, "status", "off")?;
    tmux.set_option(PANEL_SESSION, "mouse", "on")?;
    tmux.set_window_option(PANEL_SESSION, "allow-rename", "off")?;
    tmux.set_window_option(PANEL_SESSION, "allow-set-title", "off")?;
    tmux.set_window_option(PANEL_SESSION, "automatic-rename", "off")?;
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
    let content_pane_ids = split_content_grid(&tmux, &first_content_pane, &commands, layout)?;
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

fn split_content_grid(
    tmux: &Tmux,
    first_content_pane: &str,
    commands: &[String],
    layout: Layout,
) -> anyhow::Result<Vec<String>> {
    match layout {
        Layout::TwoByTwo => split_two_by_two(tmux, first_content_pane, commands),
        Layout::ThreeByThree => split_three_by_three(tmux, first_content_pane, commands),
    }
}

fn split_two_by_two(
    tmux: &Tmux,
    first_content_pane: &str,
    commands: &[String],
) -> anyhow::Result<Vec<String>> {
    let top_left = first_content_pane.to_owned();
    let top_right =
        tmux.split_window_right_percent_with_id(&top_left, 50, command_at(commands, 1)?)?;
    let bottom_left = tmux.split_window_percent_with_id(&top_left, 50, command_at(commands, 2)?)?;
    let bottom_right =
        tmux.split_window_percent_with_id(&top_right, 50, command_at(commands, 3)?)?;
    Ok(vec![top_left, top_right, bottom_left, bottom_right])
}

fn split_three_by_three(
    tmux: &Tmux,
    first_content_pane: &str,
    commands: &[String],
) -> anyhow::Result<Vec<String>> {
    let top_left = first_content_pane.to_owned();
    let top_middle =
        tmux.split_window_right_percent_with_id(&top_left, 67, command_at(commands, 1)?)?;
    let top_right =
        tmux.split_window_right_percent_with_id(&top_middle, 50, command_at(commands, 2)?)?;
    let middle_left = tmux.split_window_percent_with_id(&top_left, 67, command_at(commands, 3)?)?;
    let middle_middle =
        tmux.split_window_percent_with_id(&top_middle, 67, command_at(commands, 4)?)?;
    let middle_right =
        tmux.split_window_percent_with_id(&top_right, 67, command_at(commands, 5)?)?;
    let bottom_left =
        tmux.split_window_percent_with_id(&middle_left, 50, command_at(commands, 6)?)?;
    let bottom_middle =
        tmux.split_window_percent_with_id(&middle_middle, 50, command_at(commands, 7)?)?;
    let bottom_right =
        tmux.split_window_percent_with_id(&middle_right, 50, command_at(commands, 8)?)?;
    Ok(vec![
        top_left,
        top_middle,
        top_right,
        middle_left,
        middle_middle,
        middle_right,
        bottom_left,
        bottom_middle,
        bottom_right,
    ])
}

fn command_at(commands: &[String], index: usize) -> anyhow::Result<&str> {
    commands
        .get(index)
        .map(String::as_str)
        .with_context(|| format!("panel layout missing command for slot {}", index + 1))
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

fn set_pane_key(tmux: &Tmux, pane_id: &str, key: &str) -> anyhow::Result<()> {
    tmux.set_pane_title(pane_id, key)?;
    tmux.set_pane_option(pane_id, "@stp-pane-key", key)?;
    Ok(())
}
