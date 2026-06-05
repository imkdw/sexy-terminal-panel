use std::io::{self, Write};

use stp_core::registry::{ManagedTerminal, Registry, TerminalStatus};

use crate::panel::Layout;

const DEFAULT_WIDTH: usize = 120;
const CELL_LINES: usize = 4;
const MARKER: &str = "...";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LineEnding {
    Lf,
}

impl LineEnding {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
        }
    }
}

pub(super) fn render<W: Write>(
    registry: &Registry,
    layout: Layout,
    focus: usize,
    line_ending: LineEnding,
    max_width: Option<usize>,
    writer: &mut W,
) -> io::Result<()> {
    let shape = GridShape::new(layout, max_width);
    write_line(
        writer,
        line_ending,
        &truncate_to_width("STP panel", shape.width),
    )?;
    write_line(
        writer,
        line_ending,
        &truncate_to_width(
            &format!(
                "Layout: {} | Focus slot: {}",
                layout.label(),
                focus.saturating_add(1)
            ),
            shape.width,
        ),
    )?;
    write_line(writer, line_ending, &shape.border())?;

    for row in 0..shape.rows {
        let cells = row_cells(registry, focus, shape.columns, row);
        for line_index in 0..CELL_LINES {
            writer.write_all(b"|")?;
            for cell in &cells {
                writer.write_all(fit_cell(&cell[line_index], shape.cell_width).as_bytes())?;
                writer.write_all(b"|")?;
            }
            writer.write_all(line_ending.as_str().as_bytes())?;
        }
        write_line(writer, line_ending, &shape.border())?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GridShape {
    columns: usize,
    rows: usize,
    cell_width: usize,
    width: usize,
}

impl GridShape {
    fn new(layout: Layout, max_width: Option<usize>) -> Self {
        let (columns, rows) = match layout {
            Layout::TwoByTwo => (2, 2),
            Layout::ThreeByThree => (3, 3),
        };
        let requested_width = max_width
            .filter(|width| *width > 0)
            .unwrap_or(DEFAULT_WIDTH)
            .max(columns + 1);
        let content_width = requested_width.saturating_sub(columns + 1);
        let cell_width = content_width.saturating_div(columns).max(1);
        let width = cell_width
            .saturating_mul(columns)
            .saturating_add(columns)
            .saturating_add(1);
        Self {
            columns,
            rows,
            cell_width,
            width,
        }
    }

    fn border(self) -> String {
        let mut border = String::with_capacity(self.width);
        border.push('+');
        for _ in 0..self.columns {
            border.push_str(&"-".repeat(self.cell_width));
            border.push('+');
        }
        border
    }
}

fn row_cells(
    registry: &Registry,
    focus: usize,
    columns: usize,
    row: usize,
) -> Vec<[String; CELL_LINES]> {
    let mut cells = Vec::with_capacity(columns);
    for column in 0..columns {
        let slot_index = row.saturating_mul(columns).saturating_add(column);
        let marker = if slot_index == focus { ">" } else { " " };
        let slot_number = slot_index.saturating_add(1);
        let cell = registry.terminals.get(slot_index).map_or_else(
            || empty_cell(marker, slot_number),
            |terminal| terminal_cell(marker, slot_number, terminal),
        );
        cells.push(cell);
    }
    cells
}

fn empty_cell(marker: &str, slot_number: usize) -> [String; CELL_LINES] {
    [
        format!("{marker}{slot_number}: <empty>"),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn terminal_cell(
    marker: &str,
    slot_number: usize,
    terminal: &ManagedTerminal,
) -> [String; CELL_LINES] {
    let name = terminal
        .workspace_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("<workspace>");
    let branch = terminal.branch_name.as_deref().unwrap_or("non-git");
    [
        format!("{marker}{slot_number}: {}", display_text(name)),
        format!(" {}", display_text(branch)),
        format!(" {}", terminal.terminal_id),
        format!(" {}", status_label(terminal.status)),
    ]
}

fn fit_cell(line: &str, width: usize) -> String {
    let fitted = truncate_to_width(line, width);
    let padding = width.saturating_sub(fitted.chars().count());
    format!("{fitted}{}", " ".repeat(padding))
}

fn write_line<W: Write>(writer: &mut W, line_ending: LineEnding, line: &str) -> io::Result<()> {
    writer.write_all(line.as_bytes())?;
    writer.write_all(line_ending.as_str().as_bytes())
}

pub(super) fn truncate_to_width(line: &str, width: usize) -> String {
    let char_count = line.chars().count();
    if width == 0 {
        return String::new();
    }
    if char_count <= width {
        return line.to_owned();
    }
    if width <= MARKER.len() {
        return MARKER.chars().take(width).collect();
    }
    let prefix_width = width.saturating_sub(MARKER.len());
    let prefix = line.chars().take(prefix_width).collect::<String>();
    format!("{prefix}{MARKER}")
}

pub(super) fn display_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect()
}

const fn status_label(status: TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Starting => "starting",
        TerminalStatus::Live => "live",
        TerminalStatus::Stale => "stale",
        TerminalStatus::Exited => "exited",
    }
}
