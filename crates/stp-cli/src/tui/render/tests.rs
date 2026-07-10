use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Position;
use stp_core::ids::TerminalId;

use super::*;
use crate::tui::layout;
use crate::tui::state::{GridKind, PanelState, SessionEntry};

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

fn symbol_at(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> &str {
    terminal
        .backend()
        .buffer()
        .cell(Position::new(x, y))
        .map_or("", ratatui::buffer::Cell::symbol)
}

#[test]
fn sidebar_and_empty_grid_render_chrome() {
    let mut state = PanelState::new(GridKind::TwoByTwo);
    state.sessions.push(SessionEntry {
        id: TerminalId::from_uuid(uuid::Uuid::from_u128(1)),
        workspace: "worktree-a".to_owned(),
        branch: "main".to_owned(),
    });
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test backend terminal");
    terminal
        .draw(|frame| {
            let regions = layout::compute(frame.area(), &state);
            draw(frame, &state, &regions, &NoScreens);
        })
        .expect("draw");
    let text = buffer_text(&terminal);
    assert!(text.contains("STP sessions"), "sidebar header missing");
    assert!(text.contains("+ new"), "new button missing");
    assert!(text.contains("[2x2]"), "grid toggle missing");
    assert!(text.contains("worktree-a/main"), "session label missing");
    assert!(text.contains("slot 1"), "empty tile title missing");
    assert!(text.contains('빈'), "empty slot placeholder missing");
}

#[test]
fn grid_render_draws_visible_pane_separators() {
    let state = PanelState::new(GridKind::ThreeByThree);
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("test backend terminal");
    let mut regions = None;
    terminal
        .draw(|frame| {
            let computed = layout::compute(frame.area(), &state);
            regions = Some(computed.clone());
            draw(frame, &state, &computed, &NoScreens);
        })
        .expect("draw");
    let regions = regions.expect("regions captured");
    let first = regions.tiles[0];
    let second = regions.tiles[1];
    let fourth = regions.tiles[3];
    let vertical_x = first.outer.right();
    let horizontal_y = first.outer.bottom();

    assert_eq!(
        second.outer.x,
        vertical_x + 1,
        "adjacent columns should leave one separator cell"
    );
    assert_eq!(
        fourth.outer.y,
        horizontal_y + 1,
        "adjacent rows should leave one separator cell"
    );
    assert_eq!(
        symbol_at(&terminal, vertical_x, first.body.y),
        "│",
        "vertical pane separator missing"
    );
    assert_eq!(
        symbol_at(&terminal, first.body.x, horizontal_y),
        "─",
        "horizontal pane separator missing"
    );
    assert_eq!(
        symbol_at(&terminal, vertical_x, horizontal_y),
        "┼",
        "pane separator crossing missing"
    );
}
