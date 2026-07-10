//! 렌더: 사이드바 + pane 그리드 + 드래그 고스트. `vt100::Screen` → `ratatui` 셀은 수동 매핑
//! (`tui-term` 은 `ratatui` 0.30 미지원이라 미사용).
//! ponytail: 수동 셀 매핑 ~70줄. `tui-term` 이 0.30 지원하면 교체 가능.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use stp_core::ids::TerminalId;

use super::layout::{Regions, Tile};
use super::state::{GridKind, PanelState};

/// pane 본문에 그릴 vt100 화면 조회원. `BrokerLink` 가 구현하고, 테스트는 빈 구현으로 대체.
pub trait ScreenSource {
    fn screen(&self, id: &TerminalId) -> Option<&vt100::Screen>;
}

pub fn draw(frame: &mut Frame, state: &PanelState, regions: &Regions, screens: &dyn ScreenSource) {
    render_sidebar(frame.buffer_mut(), state, regions);
    render_grid(frame, state, regions, screens);
    render_drag_ghost(frame.buffer_mut(), state, regions);
    place_cursor(frame, state, regions, screens);
}

fn render_sidebar(buf: &mut Buffer, state: &PanelState, regions: &Regions) {
    let sidebar = regions.sidebar;
    let header_style = Style::default().add_modifier(Modifier::BOLD);
    buf.set_stringn(
        sidebar.x + 1,
        sidebar.y,
        "STP sessions",
        usize::from(sidebar.width.saturating_sub(1)),
        header_style,
    );

    for (index, row) in regions.rows.iter().enumerate() {
        let Some(entry) = state.sessions.get(index) else {
            continue;
        };
        let focused = state
            .focused_terminal()
            .is_some_and(|id| *id == entry.id);
        let dot = if focused { "◉" } else { "●" };
        let short: String = entry.id.to_string().chars().take(6).collect();
        let label = format!("{dot} {short} {}/{}", entry.workspace, entry.branch);
        let label_width = usize::from(row.width.saturating_sub(3));
        let style = if focused {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        buf.set_stringn(row.x, row.y, &label, label_width, style);
        let close = regions.close_buttons[index];
        buf.set_stringn(close.x, close.y, "[x]", 3, Style::default().fg(Color::Red));
    }

    buf.set_stringn(
        regions.new_button.x,
        regions.new_button.y,
        "+ new",
        5,
        Style::default().fg(Color::Green),
    );
    let active = |kind: GridKind| {
        if state.grid == kind {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        }
    };
    buf.set_stringn(
        regions.grid_2x2.x,
        regions.grid_2x2.y,
        "[2x2]",
        5,
        active(GridKind::TwoByTwo),
    );
    buf.set_stringn(
        regions.grid_3x3.x,
        regions.grid_3x3.y,
        "[3x3]",
        5,
        active(GridKind::ThreeByThree),
    );
    let count = format!("{} live", state.sessions.len());
    buf.set_stringn(
        regions.count_line.x,
        regions.count_line.y,
        &count,
        usize::from(regions.count_line.width),
        Style::default().fg(Color::DarkGray),
    );
    buf.set_stringn(
        regions.hint_line.x,
        regions.hint_line.y,
        "q quit  tab focus",
        usize::from(regions.hint_line.width),
        Style::default().fg(Color::DarkGray),
    );
}

fn render_grid(frame: &mut Frame, state: &PanelState, regions: &Regions, screens: &dyn ScreenSource) {
    for (index, tile) in regions.tiles.iter().enumerate() {
        let focused = state.focus == Some(index);
        render_title(frame.buffer_mut(), state, index, *tile, focused);
        render_body(frame.buffer_mut(), state, index, *tile, screens);
    }
}

fn render_title(buf: &mut Buffer, state: &PanelState, slot: usize, tile: Tile, focused: bool) {
    let label = state
        .slot_label(slot)
        .unwrap_or_else(|| format!("slot {}", slot + 1));
    let style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray)
    };
    // 제목바 전체를 배경색으로 칠해 드래그 핸들임을 시각화.
    for x in tile.title.x..tile.title.right() {
        if let Some(cell) = buf.cell_mut(Position::new(x, tile.title.y)) {
            cell.set_symbol(" ");
            cell.set_style(style);
        }
    }
    buf.set_stringn(
        tile.title.x,
        tile.title.y,
        &label,
        usize::from(tile.title.width),
        style,
    );
}

fn render_body(
    buf: &mut Buffer,
    state: &PanelState,
    slot: usize,
    tile: Tile,
    screens: &dyn ScreenSource,
) {
    let screen = state
        .slots
        .get(slot)
        .and_then(Option::as_ref)
        .and_then(|id| screens.screen(id));
    if let Some(screen) = screen {
        render_screen(screen, tile.body, buf);
    } else {
        buf.set_stringn(
            tile.body.x,
            tile.body.y,
            "빈 슬롯 - 사이드바에서 세션 선택",
            usize::from(tile.body.width),
            Style::default().fg(Color::DarkGray),
        );
    }
}

/// `vt100::Screen` 의 셀을 `ratatui` 버퍼로 복사.
fn render_screen(screen: &vt100::Screen, area: Rect, buf: &mut Buffer) {
    let (rows, cols) = screen.size();
    for row in 0..rows.min(area.height) {
        for col in 0..cols.min(area.width) {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            let Some(target) = buf.cell_mut(Position::new(area.x + col, area.y + row)) else {
                continue;
            };
            if cell.is_wide_continuation() {
                target.set_symbol("");
                continue;
            }
            let contents = cell.contents();
            target.set_symbol(if contents.is_empty() { " " } else { contents });
            target.set_style(cell_style(cell));
        }
    }
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut modifiers = Modifier::empty();
    if cell.bold() {
        modifiers |= Modifier::BOLD;
    }
    if cell.dim() {
        modifiers |= Modifier::DIM;
    }
    if cell.italic() {
        modifiers |= Modifier::ITALIC;
    }
    if cell.underline() {
        modifiers |= Modifier::UNDERLINED;
    }
    if cell.inverse() {
        modifiers |= Modifier::REVERSED;
    }
    Style::default()
        .fg(convert_color(cell.fgcolor()))
        .bg(convert_color(cell.bgcolor()))
        .add_modifier(modifiers)
}

const fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn render_drag_ghost(buf: &mut Buffer, state: &PanelState, regions: &Regions) {
    let Some(drag) = state.drag else {
        return;
    };
    let label = state
        .slot_label(drag.from_slot)
        .unwrap_or_else(|| format!("slot {}", drag.from_slot + 1));
    let ghost = format!("⇄ {label}");
    let (x, y) = drag.cursor;
    let max = usize::from(regions.content.right().saturating_sub(x));
    buf.set_stringn(
        x,
        y,
        &ghost,
        max,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
}

fn place_cursor(frame: &mut Frame, state: &PanelState, regions: &Regions, screens: &dyn ScreenSource) {
    let Some(slot) = state.focus else {
        return;
    };
    let Some(tile) = regions.tiles.get(slot) else {
        return;
    };
    let Some(id) = state.slots.get(slot).and_then(Option::as_ref) else {
        return;
    };
    let Some(screen) = screens.screen(id) else {
        return;
    };
    if screen.hide_cursor() {
        return;
    }
    let (row, col) = screen.cursor_position();
    if col < tile.body.width && row < tile.body.height {
        frame.set_cursor_position(Position::new(tile.body.x + col, tile.body.y + row));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::layout;
    use crate::tui::state::{PanelState, SessionEntry};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    struct NoScreens;
    impl ScreenSource for NoScreens {
        fn screen(&self, _id: &TerminalId) -> Option<&vt100::Screen> {
            None
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn sidebar_and_empty_grid_render_chrome() {
        let mut state = PanelState::new(GridKind::TwoByTwo);
        state.sessions.push(SessionEntry {
            id: TerminalId::from_uuid(uuid::Uuid::from_u128(1)),
            workspace: "worktree-a".to_owned(),
            branch: "main".to_owned(),
        });
        let mut terminal =
            Terminal::new(TestBackend::new(100, 30)).expect("test backend terminal");
        terminal
            .draw(|frame| {
                let regions = layout::compute(frame.area(), &state);
                draw(frame, &state, &regions, &NoScreens);
            })
            .expect("draw");
        // 넓은 글자는 다음 셀이 공백으로 채워져 붙지 않으므로, 어설션은 ASCII 크롬 위주로.
        let text = buffer_text(&terminal);
        assert!(text.contains("STP sessions"), "sidebar header missing");
        assert!(text.contains("+ new"), "new button missing");
        assert!(text.contains("[2x2]"), "grid toggle missing");
        assert!(text.contains("worktree-a/main"), "session label missing");
        assert!(text.contains("slot 1"), "empty tile title missing");
        assert!(text.contains('빈'), "empty slot placeholder missing");
    }
}
