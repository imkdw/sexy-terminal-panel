use std::io;

use stp_core::registry::Registry;

use super::Layout;

mod grid;

#[cfg(test)]
mod grid_tests;

pub fn render_once(registry: &Registry, layout: Layout) -> io::Result<String> {
    let mut buffer = Vec::new();
    grid::render(registry, layout, 0, grid::LineEnding::Lf, None, &mut buffer)?;
    String::from_utf8(buffer).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
