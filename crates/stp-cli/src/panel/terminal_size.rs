use std::fs::File;
use std::process::{Command, Stdio};

use stp_tmux::adapter::TmuxWindowSize;

pub(super) fn current() -> Option<TmuxWindowSize> {
    from_env().or_else(from_tty).or_else(from_tmux)
}

fn from_env() -> Option<TmuxWindowSize> {
    let cols = std::env::var("COLUMNS").ok()?;
    let rows = std::env::var("LINES").ok()?;
    parse_cols_rows(&cols, &rows)
}

fn from_tty() -> Option<TmuxWindowSize> {
    let tty = File::open("/dev/tty").ok()?;
    let output = Command::new("stty")
        .arg("size")
        .stdin(Stdio::from(tty))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    parse_rows_cols(&text)
}

fn from_tmux() -> Option<TmuxWindowSize> {
    std::env::var("TMUX").ok()?;
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{pane_width} #{pane_height}"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    parse_cols_rows_text(&text)
}

pub(super) fn parse_rows_cols(text: &str) -> Option<TmuxWindowSize> {
    let mut parts = text.split_whitespace();
    let rows = parts.next()?;
    let cols = parts.next()?;
    parse_cols_rows(cols, rows)
}

pub(super) fn parse_cols_rows_text(text: &str) -> Option<TmuxWindowSize> {
    let mut parts = text.split_whitespace();
    let cols = parts.next()?;
    let rows = parts.next()?;
    parse_cols_rows(cols, rows)
}

fn parse_cols_rows(cols: &str, rows: &str) -> Option<TmuxWindowSize> {
    let cols = cols.parse().ok()?;
    let rows = rows.parse().ok()?;
    valid_panel_size(cols, rows)
}

const fn valid_panel_size(cols: usize, rows: usize) -> Option<TmuxWindowSize> {
    if cols >= 80 && rows >= 20 {
        Some(TmuxWindowSize::new(cols, rows))
    } else {
        None
    }
}
