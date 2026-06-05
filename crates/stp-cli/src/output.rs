use std::io::{self, Write};

pub fn stdout_line(line: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{line}")
}

pub fn stdout_text(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    write!(stdout, "{text}")
}
