use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::{Color, Style};

use crate::tui::layout::{Regions, Tile};
use crate::tui::state::GridKind;

const VERTICAL: &str = "│";
const HORIZONTAL: &str = "─";
const CROSS: &str = "┼";

pub(super) fn render_separators(buf: &mut Buffer, grid: GridKind, regions: &Regions) {
    let (cols, rows) = grid.dims();
    let style = Style::default().fg(Color::Gray);

    for row in 0..rows {
        for col in 0..cols.saturating_sub(1) {
            let Some(tile) = tile_at(regions, cols, row, col) else {
                continue;
            };
            let x = tile.outer.right();
            draw_vertical(buf, x, regions.content.y, regions.content.bottom(), style);
        }
    }

    for row in 0..rows.saturating_sub(1) {
        let Some(tile) = tile_at(regions, cols, row, 0) else {
            continue;
        };
        let y = tile.outer.bottom();
        draw_horizontal(buf, regions.content.x, regions.content.right(), y, style);
    }
}

fn tile_at(regions: &Regions, cols: u16, row: u16, col: u16) -> Option<Tile> {
    let index = usize::from(row)
        .checked_mul(usize::from(cols))?
        .checked_add(usize::from(col))?;
    regions.tiles.get(index).copied()
}

fn draw_vertical(buf: &mut Buffer, x: u16, top: u16, bottom: u16, style: Style) {
    for y in top..bottom {
        let Some(cell) = buf.cell_mut(Position::new(x, y)) else {
            continue;
        };
        cell.set_symbol(VERTICAL);
        cell.set_style(style);
    }
}

fn draw_horizontal(buf: &mut Buffer, left: u16, right: u16, y: u16, style: Style) {
    for x in left..right {
        let Some(cell) = buf.cell_mut(Position::new(x, y)) else {
            continue;
        };
        if cell.symbol() == VERTICAL {
            cell.set_symbol(CROSS);
        } else {
            cell.set_symbol(HORIZONTAL);
        }
        cell.set_style(style);
    }
}
