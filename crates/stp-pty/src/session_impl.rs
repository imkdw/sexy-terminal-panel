use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use stp_core::ids::TerminalId;
use stp_core::protocol::{ServerEvent, SessionSummary};
use stp_core::registry::TerminalStatus;

use crate::error::BrokerError;
use crate::session::{
    BrokerSession, DEFAULT_COLS, DEFAULT_ROWS, SCROLLBACK_LINES, lock, status_label,
};

impl BrokerSession {
    pub fn spawn(
        terminal_id: TerminalId,
        workspace_path: &Path,
        shell: Option<String>,
    ) -> Result<Arc<Self>, BrokerError> {
        let system = native_pty_system();
        let pair = system
            .openpty(PtySize {
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|source| BrokerError::Pty(source.to_string()))?;
        let mut command = CommandBuilder::new(shell.unwrap_or_else(default_shell));
        command.cwd(workspace_path.as_os_str());
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|source| BrokerError::Pty(source.to_string()))?;
        let process_id = child.process_id();
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|source| BrokerError::Pty(source.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|source| BrokerError::Pty(source.to_string()))?;
        let session = Arc::new(Self {
            terminal_id,
            process_id,
            seq: std::sync::atomic::AtomicU64::new(0),
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            child: Mutex::new(child),
            parser: Mutex::new(vt100::Parser::new(
                DEFAULT_ROWS,
                DEFAULT_COLS,
                SCROLLBACK_LINES,
            )),
            status: Mutex::new(TerminalStatus::Live),
            subscribers: Mutex::new(Vec::new()),
        });
        let reader_session = Arc::clone(&session);
        thread::Builder::new()
            .name("stp-pty-reader".to_owned())
            .spawn(move || read_output(&reader_session, &mut reader))?;
        Ok(session)
    }

    pub fn add_subscriber(&self, tx: Sender<ServerEvent>) -> Result<(), BrokerError> {
        lock(&self.subscribers, "subscribers")?.push(tx);
        Ok(())
    }

    pub fn input(&self, data: &[u8]) -> Result<(), BrokerError> {
        let mut writer = lock(&self.writer, "writer")?;
        writer.write_all(data)?;
        writer.flush()?;
        drop(writer);
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), BrokerError> {
        lock(&self.master, "master")?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|source| BrokerError::Pty(source.to_string()))?;
        lock(&self.parser, "parser")?
            .screen_mut()
            .set_size(rows, cols);
        Ok(())
    }

    pub fn snapshot(&self, lines: Option<u16>) -> Result<ServerEvent, BrokerError> {
        let parser = lock(&self.parser, "parser")?;
        let (rows, cols) = parser.screen().size();
        let text = limited_lines(parser.screen().contents(), lines);
        drop(parser);
        Ok(ServerEvent::Snapshot {
            terminal_id: self.terminal_id.clone(),
            cols,
            rows,
            text,
        })
    }

    pub fn terminate(&self) -> Result<(), BrokerError> {
        let exit_code = self.stop_child()?;
        self.mark_exited(exit_code)?;
        Ok(())
    }

    pub fn status(&self) -> Result<TerminalStatus, BrokerError> {
        lock(&self.status, "status").map(|status| *status)
    }

    pub fn summary(&self) -> Result<SessionSummary, BrokerError> {
        Ok(SessionSummary {
            terminal_id: self.terminal_id.clone(),
            status: status_label(self.status()?),
        })
    }

    fn process_output(&self, data: &[u8]) -> Result<(), BrokerError> {
        lock(&self.parser, "parser")?.process(data);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        self.broadcast(&ServerEvent::output_bytes(
            self.terminal_id.clone(),
            seq,
            data,
        ))
    }

    fn broadcast(&self, event: &ServerEvent) -> Result<(), BrokerError> {
        lock(&self.subscribers, "subscribers")?.retain(|tx| tx.send(event.clone()).is_ok());
        Ok(())
    }

    fn mark_exited(&self, exit_code: Option<i32>) -> Result<(), BrokerError> {
        let mut status = lock(&self.status, "status")?;
        if *status == TerminalStatus::Exited {
            return Ok(());
        }
        *status = TerminalStatus::Exited;
        drop(status);
        self.broadcast(&ServerEvent::Exit {
            terminal_id: self.terminal_id.clone(),
            exit_code,
        })
    }

    fn stop_child(&self) -> Result<Option<i32>, BrokerError> {
        let mut child = lock(&self.child, "child")?;
        if let Some(status) = child.try_wait()? {
            return Ok(to_protocol_exit_code(status.exit_code()));
        }
        child.kill()?;
        child
            .wait()
            .map(|status| to_protocol_exit_code(status.exit_code()))
            .map_err(Into::into)
    }

    fn try_reap_child(&self) -> Result<Option<i32>, BrokerError> {
        let mut child = lock(&self.child, "child")?;
        child
            .try_wait()
            .map(|status| status.and_then(|status| to_protocol_exit_code(status.exit_code())))
            .map_err(Into::into)
    }
}

fn read_output(session: &BrokerSession, reader: &mut Box<dyn Read + Send>) {
    let mut buffer = [0_u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(bytes) => {
                if session.process_output(&buffer[..bytes]).is_err() {
                    break;
                }
            }
        }
    }
    let exit_code = session.try_reap_child().ok().flatten();
    let _ = session.mark_exited(exit_code);
}

fn to_protocol_exit_code(exit_code: u32) -> Option<i32> {
    i32::try_from(exit_code).ok()
}

fn limited_lines(text: String, lines: Option<u16>) -> String {
    let Some(lines) = lines else {
        return text;
    };
    let rows: Vec<&str> = text.lines().collect();
    let start = rows.len().saturating_sub(usize::from(lines));
    rows[start..].join("\n")
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_owned())
}
