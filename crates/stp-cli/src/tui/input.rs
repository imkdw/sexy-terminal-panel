//! `crossterm` 이벤트 → 액션 매핑 + `KeyEvent` → PTY 바이트 인코딩.
//! 순수 함수라 broker/터미널 없이 테스트 가능.

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::layout::{Hit, Regions};
use super::state::{GridKind, PanelState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    None,
    Quit,
    FocusSlot(usize),
    PlaceSession(usize),
    CloseSession(usize),
    Spawn,
    SetGrid(GridKind),
    DragStart(usize),
    DragMove(u16, u16),
    /// 드래그 종료. target 이 슬롯이면 swap, None 이면 취소.
    DragEnd(Option<usize>),
    /// 포커스 pane 으로 포워드할 인코딩된 바이트.
    Key(Vec<u8>),
}

pub fn map(event: &Event, regions: &Regions, state: &PanelState) -> Action {
    match event {
        Event::Key(key) => map_key(*key, state),
        Event::Mouse(mouse) => map_mouse(*mouse, regions, state),
        _ => Action::None,
    }
}

fn map_key(key: KeyEvent, state: &PanelState) -> Action {
    if key.kind == KeyEventKind::Release {
        return Action::None;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    // Chrome 코드: 포커스 여부와 무관하게 가로챔.
    if ctrl {
        match key.code {
            KeyCode::Char('q') => return Action::Quit,
            KeyCode::Char('n') => return Action::Spawn,
            KeyCode::Char('g') => return Action::SetGrid(state.grid.toggled()),
            _ => {}
        }
    }
    // pane 이 있으면 입력 포워드, 없으면 바로 종료 편의키.
    if state.focused_terminal().is_some() {
        encode_key(key).map_or(Action::None, Action::Key)
    } else if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
        Action::Quit
    } else {
        Action::None
    }
}

fn map_mouse(mouse: MouseEvent, regions: &Regions, state: &PanelState) -> Action {
    let (x, y) = (mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => match regions.hit(x, y) {
            Hit::CloseButton(index) => Action::CloseSession(index),
            Hit::SessionRow(index) => Action::PlaceSession(index),
            Hit::NewButton => Action::Spawn,
            Hit::Grid(kind) => Action::SetGrid(kind),
            Hit::TileTitle(slot) => Action::DragStart(slot),
            Hit::TileBody(slot) => Action::FocusSlot(slot),
            Hit::Outside => Action::None,
        },
        MouseEventKind::Drag(MouseButton::Left) if state.drag.is_some() => {
            Action::DragMove(x, y)
        }
        MouseEventKind::Up(MouseButton::Left) if state.drag.is_some() => {
            match regions.hit(x, y) {
                Hit::TileTitle(slot) | Hit::TileBody(slot) => Action::DragEnd(Some(slot)),
                _ => Action::DragEnd(None),
            }
        }
        _ => Action::None,
    }
}

/// `KeyEvent` → 터미널로 보낼 바이트 시퀀스.
pub fn encode_key(key: KeyEvent) -> Option<Vec<u8>> {
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let mut bytes = match key.code {
        KeyCode::Char(c) => encode_char(c, ctrl),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(n) => encode_function(n)?,
        _ => return None,
    };
    // ALT 는 ESC 접두사(메타).
    if alt && !matches!(key.code, KeyCode::Esc) {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(0x1b);
        prefixed.append(&mut bytes);
        return Some(prefixed);
    }
    Some(bytes)
}

fn encode_char(c: char, ctrl: bool) -> Vec<u8> {
    if ctrl {
        // Ctrl+A..Z → 0x01..0x1a, 그 외 몇몇 제어문자.
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_lowercase() {
            return vec![(lower as u8) - b'a' + 1];
        }
        return match c {
            ' ' | '@' => vec![0],
            '[' => vec![0x1b],
            '\\' => vec![0x1c],
            ']' => vec![0x1d],
            '^' => vec![0x1e],
            '_' => vec![0x1f],
            _ => c.to_string().into_bytes(),
        };
    }
    c.to_string().into_bytes()
}

fn encode_function(n: u8) -> Option<Vec<u8>> {
    match n {
        1 => Some(b"\x1bOP".to_vec()),
        2 => Some(b"\x1bOQ".to_vec()),
        3 => Some(b"\x1bOR".to_vec()),
        4 => Some(b"\x1bOS".to_vec()),
        5 => Some(b"\x1b[15~".to_vec()),
        6 => Some(b"\x1b[17~".to_vec()),
        7 => Some(b"\x1b[18~".to_vec()),
        8 => Some(b"\x1b[19~".to_vec()),
        9 => Some(b"\x1b[20~".to_vec()),
        10 => Some(b"\x1b[21~".to_vec()),
        11 => Some(b"\x1b[23~".to_vec()),
        12 => Some(b"\x1b[24~".to_vec()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new_with_kind(code, mods, KeyEventKind::Press)
    }

    #[test]
    fn plain_char_is_utf8() {
        assert_eq!(
            encode_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(b"a".to_vec())
        );
        assert_eq!(
            encode_key(key(KeyCode::Char('한'), KeyModifiers::NONE)),
            Some("한".as_bytes().to_vec())
        );
    }

    #[test]
    fn ctrl_char_is_control_byte() {
        assert_eq!(
            encode_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![3])
        );
        assert_eq!(
            encode_key(key(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Some(vec![1])
        );
    }

    #[test]
    fn special_keys_encode_to_xterm() {
        assert_eq!(
            encode_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(b"\r".to_vec())
        );
        assert_eq!(
            encode_key(key(KeyCode::Up, KeyModifiers::NONE)),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_key(key(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(vec![0x7f])
        );
    }

    #[test]
    fn alt_char_gets_escape_prefix() {
        assert_eq!(
            encode_key(key(KeyCode::Char('b'), KeyModifiers::ALT)),
            Some(vec![0x1b, b'b'])
        );
    }

    #[test]
    fn ctrl_q_quits_even_with_focus() {
        let mut state = PanelState::new(GridKind::TwoByTwo);
        // 포커스가 있어도 Ctrl+Q 는 chrome.
        state.sessions.push(super::super::state::SessionEntry {
            id: stp_core::ids::TerminalId::parse(&uuid::Uuid::from_u128(1).to_string())
                .expect("uuid"),
            workspace: "ws".to_owned(),
            branch: "main".to_owned(),
        });
        state.place_session(0);
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::CONTROL), &state),
            Action::Quit
        );
        // 포커스가 있으면 평범한 q 는 pane 으로 포워드.
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::NONE), &state),
            Action::Key(b"q".to_vec())
        );
    }

    #[test]
    fn bare_q_quits_when_no_focus() {
        let state = PanelState::new(GridKind::TwoByTwo);
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::NONE), &state),
            Action::Quit
        );
    }
}
