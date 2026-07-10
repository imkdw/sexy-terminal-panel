use anyhow::Context;
use stp_tmux::adapter::Tmux;

use super::Layout;

pub(super) fn split_content_grid(
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
