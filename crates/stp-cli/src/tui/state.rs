//! 패널 순수 상태 + 전이. broker/터미널 없이 단위 테스트 가능하게 유지.

use stp_core::ids::TerminalId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridKind {
    TwoByTwo,
    ThreeByThree,
}

impl GridKind {
    pub const fn dims(self) -> (u16, u16) {
        match self {
            Self::TwoByTwo => (2, 2),
            Self::ThreeByThree => (3, 3),
        }
    }

    pub const fn cells(self) -> usize {
        let (cols, rows) = self.dims();
        (cols as usize) * (rows as usize)
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::TwoByTwo => Self::ThreeByThree,
            Self::ThreeByThree => Self::TwoByTwo,
        }
    }
}

/// 사이드바 한 행.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionEntry {
    pub id: TerminalId,
    pub workspace: String,
    pub branch: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Drag {
    pub from_slot: usize,
    pub cursor: (u16, u16),
}

#[derive(Debug)]
pub struct PanelState {
    pub sessions: Vec<SessionEntry>,
    pub grid: GridKind,
    pub slots: Vec<Option<TerminalId>>,
    pub focus: Option<usize>,
    pub drag: Option<Drag>,
}

impl PanelState {
    pub fn new(grid: GridKind) -> Self {
        Self {
            sessions: Vec::new(),
            grid,
            slots: vec![None; grid.cells()],
            focus: None,
            drag: None,
        }
    }

    pub fn focused_terminal(&self) -> Option<&TerminalId> {
        self.focus.and_then(|slot| self.slots.get(slot)?.as_ref())
    }

    /// 슬롯에 배치된 terminal 의 사이드바 라벨(제목바용).
    pub fn slot_label(&self, slot: usize) -> Option<String> {
        let id = self.slots.get(slot)?.as_ref()?;
        self.sessions
            .iter()
            .find(|entry| entry.id == *id)
            .map(|entry| format!("{}/{}", entry.workspace, entry.branch))
    }

    fn slot_of(&self, id: &TerminalId) -> Option<usize> {
        self.slots.iter().position(|slot| slot.as_ref() == Some(id))
    }

    fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(Option::is_none)
    }

    /// 사이드바에서 세션 선택: 보이면 포커스, 아니면 첫 빈 슬롯, 꽉 차면 최우측 교체.
    /// 배치/포커스된 슬롯 인덱스를 반환.
    pub fn place_session(&mut self, session_index: usize) -> Option<usize> {
        let id = self.sessions.get(session_index)?.id.clone();
        let slot = if let Some(slot) = self.slot_of(&id) {
            slot
        } else if let Some(empty) = self.first_empty() {
            self.slots[empty] = Some(id);
            empty
        } else {
            let last = self.slots.len().checked_sub(1)?;
            self.slots[last] = Some(id);
            last
        };
        self.focus = Some(slot);
        Some(slot)
    }

    /// 신규 스폰 세션 등록 + 첫 빈 슬롯 배치. 배치된 슬롯을 반환(없으면 None).
    pub fn add_session(&mut self, entry: SessionEntry) -> Option<usize> {
        let id = entry.id.clone();
        if !self.sessions.iter().any(|existing| existing.id == id) {
            self.sessions.push(entry);
        }
        let slot = self.first_empty()?;
        self.slots[slot] = Some(id);
        self.focus = Some(slot);
        Some(slot)
    }

    /// 세션 종료(Exit/Terminate): 사이드바/슬롯에서 제거, 포커스 정리.
    pub fn remove_session(&mut self, id: &TerminalId) {
        self.sessions.retain(|entry| entry.id != *id);
        for slot in &mut self.slots {
            if slot.as_ref() == Some(id) {
                *slot = None;
            }
        }
        self.normalize_focus();
    }

    pub fn focus_slot(&mut self, slot: usize) {
        if self.slots.get(slot).is_some_and(Option::is_some) {
            self.focus = Some(slot);
        }
    }

    /// 두 슬롯 배치 교환(DnD). 같은 슬롯이면 무시.
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a == b || a >= self.slots.len() || b >= self.slots.len() {
            return;
        }
        self.slots.swap(a, b);
        // 드래그한 pane 을 계속 따라가도록 포커스를 목적지로 이동.
        self.focus = Some(b);
    }

    /// 2x2 ⇄ 3x3 모핑. terminal 을 슬롯 순서대로 보존, 넘치면 배치 해제(세션은 유지).
    pub fn set_grid(&mut self, grid: GridKind) {
        if grid == self.grid {
            return;
        }
        let focused_id = self.focused_terminal().cloned();
        let placed: Vec<TerminalId> = self.slots.iter().flatten().cloned().collect();
        let mut slots = vec![None; grid.cells()];
        for (target, id) in slots.iter_mut().zip(placed) {
            *target = Some(id);
        }
        self.grid = grid;
        self.slots = slots;
        self.focus = focused_id.and_then(|id| self.slot_of(&id));
        self.normalize_focus();
    }

    fn normalize_focus(&mut self) {
        let still_valid = self
            .focus
            .is_some_and(|slot| self.slots.get(slot).is_some_and(Option::is_some));
        if !still_valid {
            self.focus = self.slots.iter().position(Option::is_some);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(n: u128) -> TerminalId {
        TerminalId::parse(&uuid::Uuid::from_u128(n).to_string()).expect("valid uuid")
    }

    fn entry(n: u128) -> SessionEntry {
        SessionEntry {
            id: tid(n),
            workspace: format!("ws{n}"),
            branch: "main".to_owned(),
        }
    }

    #[test]
    fn place_session_follows_visible_then_empty_then_rightmost() {
        let mut state = PanelState::new(GridKind::TwoByTwo);
        for n in 0..3 {
            state.sessions.push(entry(n));
        }
        // 첫 배치 → 빈 슬롯 0
        assert_eq!(state.place_session(0), Some(0));
        // 이미 보이면 그 슬롯 포커스(재배치 없음)
        assert_eq!(state.place_session(0), Some(0));
        // 다음 빈 슬롯들
        assert_eq!(state.place_session(1), Some(1));
        assert_eq!(state.place_session(2), Some(2));
    }

    #[test]
    fn place_session_replaces_rightmost_when_full() {
        let mut state = PanelState::new(GridKind::TwoByTwo);
        for n in 0..5 {
            state.sessions.push(entry(n));
        }
        for n in 0..4 {
            state.place_session(n);
        }
        // 4칸 꽉 참 → 5번째는 최우측(3) 교체
        assert_eq!(state.place_session(4), Some(3));
        assert_eq!(state.slots[3], Some(tid(4)));
    }

    #[test]
    fn toggle_grid_preserves_terminals_and_drops_overflow_on_shrink() {
        let mut state = PanelState::new(GridKind::ThreeByThree);
        for n in 0..9 {
            state.sessions.push(entry(n));
            state.place_session(n as usize);
        }
        assert_eq!(state.slots.iter().flatten().count(), 9);
        state.set_grid(state.grid.toggled()); // 3x3 → 2x2
        assert_eq!(state.grid, GridKind::TwoByTwo);
        assert_eq!(state.slots.len(), 4);
        // 앞 4개 terminal 보존, 세션은 전부 남음
        assert_eq!(state.slots[0], Some(tid(0)));
        assert_eq!(state.sessions.len(), 9);
    }

    #[test]
    fn remove_session_clears_slot_and_refocuses() {
        let mut state = PanelState::new(GridKind::TwoByTwo);
        state.sessions.push(entry(0));
        state.sessions.push(entry(1));
        state.place_session(0);
        state.place_session(1);
        state.focus_slot(1);
        state.remove_session(&tid(1));
        assert!(!state.sessions.iter().any(|entry| entry.id == tid(1)));
        assert_eq!(state.slots[1], None);
        assert_eq!(state.focus, Some(0));
    }

    #[test]
    fn swap_slots_moves_focus_to_destination() {
        let mut state = PanelState::new(GridKind::TwoByTwo);
        state.sessions.push(entry(0));
        state.place_session(0); // slot 0
        state.swap_slots(0, 2);
        assert_eq!(state.slots[2], Some(tid(0)));
        assert_eq!(state.slots[0], None);
        assert_eq!(state.focus, Some(2));
    }
}
