//! 레이아웃 계산 + 마우스 히트 테스트. ratatui `Rect` 만 쓰는 순수 함수라 단위 테스트 가능.

use ratatui::layout::Rect;

use super::state::{GridKind, PanelState};

pub const SIDEBAR_WIDTH: u16 = 30;
const HEADER_HEIGHT: u16 = 1;
const FOOTER_HEIGHT: u16 = 4;
const CLOSE_WIDTH: u16 = 3; // "[x]"
const TILE_GAP: u16 = 1;

/// pane 타일 하나: 전체 칸 / 제목바(드래그 핸들) / 본문(터미널).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile {
    pub outer: Rect,
    pub title: Rect,
    pub body: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Regions {
    pub sidebar: Rect,
    pub content: Rect,
    pub rows: Vec<Rect>,
    pub close_buttons: Vec<Rect>,
    pub new_button: Rect,
    pub grid_2x2: Rect,
    pub grid_3x3: Rect,
    pub count_line: Rect,
    pub hint_line: Rect,
    pub tiles: Vec<Tile>,
}

/// 히트 테스트 결과.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hit {
    SessionRow(usize),
    CloseButton(usize),
    NewButton,
    Grid(GridKind),
    TileTitle(usize),
    TileBody(usize),
    Outside,
}

/// pane 본문 rect → PTY/파서 셀 크기(cols, rows). 0 방지로 최소 1.
pub const fn pty_size(body: Rect) -> (u16, u16) {
    let cols = if body.width == 0 { 1 } else { body.width };
    let rows = if body.height == 0 { 1 } else { body.height };
    (cols, rows)
}

const fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.right() && y >= rect.y && y < rect.bottom()
}

pub fn compute(area: Rect, state: &PanelState) -> Regions {
    let sidebar_width = SIDEBAR_WIDTH.min(area.width);
    let sidebar = Rect::new(area.x, area.y, sidebar_width, area.height);
    let content = Rect::new(
        area.x + sidebar_width,
        area.y,
        area.width.saturating_sub(sidebar_width),
        area.height,
    );

    let footer_top = sidebar.bottom().saturating_sub(FOOTER_HEIGHT);
    let rows_top = sidebar.y + HEADER_HEIGHT;
    let rows_area_height = footer_top.saturating_sub(rows_top);
    let inner_x = sidebar.x + 1;
    let inner_width = sidebar.width.saturating_sub(2);

    let mut rows = Vec::new();
    let mut close_buttons = Vec::new();
    for index in 0..state.sessions.len() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        if offset >= rows_area_height {
            break;
        }
        let y = rows_top + offset;
        let row = Rect::new(inner_x, y, inner_width, 1);
        rows.push(row);
        let close_x = row.right().saturating_sub(CLOSE_WIDTH);
        close_buttons.push(Rect::new(close_x, y, CLOSE_WIDTH, 1));
    }

    let new_button = Rect::new(inner_x, footer_top, 5, 1); // "+ new"
    let grid_2x2 = Rect::new(inner_x, footer_top + 1, 5, 1); // "[2x2]"
    let grid_3x3 = Rect::new(inner_x + 6, footer_top + 1, 5, 1); // "[3x3]"
    let count_line = Rect::new(inner_x, footer_top + 2, inner_width, 1);
    let hint_line = Rect::new(inner_x, footer_top + 3, inner_width, 1);

    let tiles = tiles(content, state.grid);

    Regions {
        sidebar,
        content,
        rows,
        close_buttons,
        new_button,
        grid_2x2,
        grid_3x3,
        count_line,
        hint_line,
        tiles,
    }
}

fn tiles(content: Rect, grid: GridKind) -> Vec<Tile> {
    let (cols, grid_rows) = grid.dims();
    let mut out = Vec::with_capacity(grid.cells());
    if content.width == 0 || content.height == 0 {
        return out;
    }
    let gap_width = cols.saturating_sub(1).saturating_mul(TILE_GAP);
    let gap_height = grid_rows.saturating_sub(1).saturating_mul(TILE_GAP);
    let tile_area_width = content.width.saturating_sub(gap_width);
    let tile_area_height = content.height.saturating_sub(gap_height);
    let cell_w = tile_area_width / cols;
    let cell_h = tile_area_height / grid_rows;
    for row in 0..grid_rows {
        for col in 0..cols {
            let x = content.x + col * (cell_w + TILE_GAP);
            let y = content.y + row * (cell_h + TILE_GAP);
            // 마지막 열/행은 나머지 픽셀을 흡수.
            let w = if col + 1 == cols {
                content.right().saturating_sub(x)
            } else {
                cell_w
            };
            let h = if row + 1 == grid_rows {
                content.bottom().saturating_sub(y)
            } else {
                cell_h
            };
            let outer = Rect::new(x, y, w, h);
            let title = Rect::new(x, y, w, 1.min(h));
            let body = Rect::new(x, y + 1.min(h), w, h.saturating_sub(1));
            out.push(Tile { outer, title, body });
        }
    }
    out
}

impl Regions {
    pub fn hit(&self, x: u16, y: u16) -> Hit {
        for (index, rect) in self.close_buttons.iter().enumerate() {
            if contains(*rect, x, y) {
                return Hit::CloseButton(index);
            }
        }
        for (index, rect) in self.rows.iter().enumerate() {
            if contains(*rect, x, y) {
                return Hit::SessionRow(index);
            }
        }
        if contains(self.new_button, x, y) {
            return Hit::NewButton;
        }
        if contains(self.grid_2x2, x, y) {
            return Hit::Grid(GridKind::TwoByTwo);
        }
        if contains(self.grid_3x3, x, y) {
            return Hit::Grid(GridKind::ThreeByThree);
        }
        for (index, tile) in self.tiles.iter().enumerate() {
            if contains(tile.title, x, y) {
                return Hit::TileTitle(index);
            }
            if contains(tile.body, x, y) {
                return Hit::TileBody(index);
            }
        }
        Hit::Outside
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::{GridKind, PanelState, SessionEntry};
    use stp_core::ids::TerminalId;

    fn tid(n: u128) -> TerminalId {
        TerminalId::parse(&uuid::Uuid::from_u128(n).to_string()).expect("valid uuid")
    }

    fn state_with(sessions: usize, grid: GridKind) -> PanelState {
        let mut state = PanelState::new(grid);
        for n in 0..sessions {
            state.sessions.push(SessionEntry {
                id: tid(n as u128),
                workspace: "ws".to_owned(),
                branch: "main".to_owned(),
            });
        }
        state
    }

    #[test]
    fn close_button_beats_row_at_same_cell() {
        let state = state_with(1, GridKind::TwoByTwo);
        let regions = compute(Rect::new(0, 0, 100, 30), &state);
        let close = regions.close_buttons[0];
        assert_eq!(regions.hit(close.x, close.y), Hit::CloseButton(0));
        // 행 본문(닫기 왼쪽)은 SessionRow
        assert_eq!(
            regions.hit(regions.sidebar.x + 2, close.y),
            Hit::SessionRow(0)
        );
    }

    #[test]
    fn footer_buttons_hit() {
        let state = state_with(0, GridKind::TwoByTwo);
        let regions = compute(Rect::new(0, 0, 100, 30), &state);
        assert_eq!(
            regions.hit(regions.new_button.x, regions.new_button.y),
            Hit::NewButton
        );
        assert_eq!(
            regions.hit(regions.grid_2x2.x, regions.grid_2x2.y),
            Hit::Grid(GridKind::TwoByTwo)
        );
        assert_eq!(
            regions.hit(regions.grid_3x3.x, regions.grid_3x3.y),
            Hit::Grid(GridKind::ThreeByThree)
        );
    }

    #[test]
    fn tile_title_and_body_split() {
        let state = state_with(0, GridKind::TwoByTwo);
        let regions = compute(Rect::new(0, 0, 100, 30), &state);
        assert_eq!(regions.tiles.len(), 4);
        let tile = regions.tiles[0];
        assert_eq!(regions.hit(tile.title.x, tile.title.y), Hit::TileTitle(0));
        assert_eq!(regions.hit(tile.body.x, tile.body.y), Hit::TileBody(0));
    }

    #[test]
    fn grid_switch_changes_tile_count() {
        let state = state_with(0, GridKind::ThreeByThree);
        let regions = compute(Rect::new(0, 0, 120, 40), &state);
        assert_eq!(regions.tiles.len(), 9);
    }
}
