use std::io;

use stp_core::registry::{Registry, RegistryStore};

mod rendering;
mod tmux_panel;

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
