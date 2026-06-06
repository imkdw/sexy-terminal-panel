use std::io;

use stp_core::registry::{Registry, RegistryStore};

mod bindings;
mod layout;
mod rendering;
mod session_sidebar;
#[cfg(test)]
mod session_sidebar_tests;
mod shell;
mod tmux_panel;
#[cfg(test)]
mod tmux_panel_tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    TwoByTwo,
    ThreeByThree,
}

impl Layout {
    pub fn parse(value: &str) -> Self {
        if value == "2x2" {
            Self::TwoByTwo
        } else {
            Self::ThreeByThree
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::TwoByTwo => "2x2",
            Self::ThreeByThree => "3x3",
        }
    }

    pub(super) const fn capacity(self) -> usize {
        match self {
            Self::TwoByTwo => 4,
            Self::ThreeByThree => 9,
        }
    }
}

pub fn render_once(registry: &Registry, layout: Layout) -> io::Result<String> {
    rendering::render_once(registry, layout)
}

pub fn run_interactive(
    store: &RegistryStore,
    layout: Layout,
    panel_socket: &str,
) -> anyhow::Result<()> {
    tmux_panel::open(store, layout, panel_socket)
}

pub fn select_from_sidebar(
    store: &RegistryStore,
    mouse_line: &str,
    panel_socket: &str,
) -> anyhow::Result<()> {
    tmux_panel::select_from_sidebar(store, mouse_line, panel_socket)
}
