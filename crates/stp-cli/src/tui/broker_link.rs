//! broker 연결 계층: control 연결(요청/응답) + event 연결(Attach 스트림).
//! 세션별 vt100 파서를 메인 스레드에 보관하고, reader thread 가 서버 이벤트를 mpsc 로 넘긴다.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result, anyhow};
use stp_core::ids::{TerminalId, WindowId};
use stp_core::protocol::{ClientRequest, ServerEvent, decode_server_frame, encode_client_frame};
use stp_pty::BrokerClient;

const SCROLLBACK_LINES: usize = 2_000;

/// `drain()` 이 메인 루프에 돌려주는 의미 이벤트(파서는 이미 반영됨).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkEvent {
    Output(TerminalId),
    Snapshot(TerminalId),
    Exited(TerminalId),
    Disconnected,
}

pub struct BrokerLink {
    control: BrokerClient,
    event_writer: UnixStream,
    events: Receiver<ServerEvent>,
    parsers: HashMap<TerminalId, vt100::Parser>,
    reader: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for BrokerLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrokerLink")
            .field("parsers", &self.parsers.len())
            .finish_non_exhaustive()
    }
}

impl BrokerLink {
    pub fn connect(socket: &Path) -> Result<Self> {
        let control = BrokerClient::connect(socket).context("connect control socket")?;
        let event_stream = UnixStream::connect(socket).context("connect event socket")?;
        let event_writer = event_stream.try_clone().context("clone event socket")?;
        let (tx, rx) = mpsc::channel();
        let reader = thread::Builder::new()
            .name("stp-tui-reader".to_owned())
            .spawn(move || read_loop(event_stream, &tx))
            .context("spawn reader thread")?;
        Ok(Self {
            control,
            event_writer,
            events: rx,
            parsers: HashMap::new(),
            reader: Some(reader),
        })
    }

    /// event 연결에 Attach 전송. Snapshot/Output 은 reader thread 로 돌아온다.
    pub fn attach(&mut self, terminal_id: &TerminalId) -> Result<()> {
        self.send_event(&ClientRequest::Attach {
            terminal_id: terminal_id.clone(),
        })
    }

    pub fn input(&mut self, terminal_id: &TerminalId, data: &[u8]) -> Result<()> {
        self.control
            .request(&ClientRequest::input_bytes(terminal_id.clone(), data))?;
        Ok(())
    }

    pub fn resize(&mut self, terminal_id: &TerminalId, cols: u16, rows: u16) -> Result<()> {
        self.control.request(&ClientRequest::Resize {
            terminal_id: terminal_id.clone(),
            cols,
            rows,
        })?;
        if let Some(parser) = self.parsers.get_mut(terminal_id) {
            parser.screen_mut().set_size(rows, cols);
        }
        Ok(())
    }

    pub fn spawn(
        &mut self,
        terminal_id: &TerminalId,
        window_id: &WindowId,
        workspace_path: PathBuf,
        shell: Option<String>,
        command: Option<Vec<String>>,
    ) -> Result<()> {
        match self.control.request(&ClientRequest::Spawn {
            terminal_id: terminal_id.clone(),
            window_id: window_id.clone(),
            workspace_path,
            shell,
            command,
        })? {
            ServerEvent::Spawned { .. } => Ok(()),
            event => Err(anyhow!("spawn failed: {event:?}")),
        }
    }

    pub fn terminate(&mut self, terminal_id: &TerminalId) -> Result<()> {
        self.control.request(&ClientRequest::Terminate {
            terminal_id: terminal_id.clone(),
        })?;
        self.parsers.remove(terminal_id);
        Ok(())
    }

    pub fn screen(&self, terminal_id: &TerminalId) -> Option<&vt100::Screen> {
        self.parsers.get(terminal_id).map(vt100::Parser::screen)
    }

    /// mpsc 를 비우며 파서를 갱신하고, 의미 이벤트 목록을 돌려준다.
    pub fn drain(&mut self) -> Vec<LinkEvent> {
        let mut out = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    if let Some(link_event) = self.apply(event) {
                        out.push(link_event);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    out.push(LinkEvent::Disconnected);
                    break;
                }
            }
        }
        out
    }

    fn apply(&mut self, event: ServerEvent) -> Option<LinkEvent> {
        match event {
            ServerEvent::Snapshot {
                terminal_id,
                cols,
                rows,
                text,
            } => {
                let parser = self
                    .parsers
                    .entry(terminal_id.clone())
                    .or_insert_with(|| vt100::Parser::new(rows, cols, SCROLLBACK_LINES));
                parser.screen_mut().set_size(rows, cols);
                // plain snapshot 은 개행에 캐리지리턴이 없어 계단현상이 나므로 보정.
                parser.process(text.replace('\n', "\r\n").as_bytes());
                Some(LinkEvent::Snapshot(terminal_id))
            }
            ServerEvent::Output { ref terminal_id, .. } => {
                let terminal_id = terminal_id.clone();
                let data = event.output_data().ok()?;
                let parser = self
                    .parsers
                    .entry(terminal_id.clone())
                    .or_insert_with(|| vt100::Parser::new(24, 80, SCROLLBACK_LINES));
                parser.process(&data);
                Some(LinkEvent::Output(terminal_id))
            }
            ServerEvent::Exit { terminal_id, .. } => {
                self.parsers.remove(&terminal_id);
                Some(LinkEvent::Exited(terminal_id))
            }
            _ => None,
        }
    }

    fn send_event(&mut self, request: &ClientRequest) -> Result<()> {
        let frame = encode_client_frame(request).context("encode event frame")?;
        self.event_writer
            .write_all(frame.as_bytes())
            .context("write event frame")?;
        self.event_writer.flush().context("flush event frame")?;
        Ok(())
    }
}

impl super::render::ScreenSource for BrokerLink {
    fn screen(&self, id: &TerminalId) -> Option<&vt100::Screen> {
        Self::screen(self, id)
    }
}

impl Drop for BrokerLink {
    fn drop(&mut self) {
        // event 소켓을 닫으면 reader thread 의 read_line 이 0 을 반환하며 종료한다.
        let _ = self.event_writer.shutdown(std::net::Shutdown::Both);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn read_loop(stream: UnixStream, tx: &mpsc::Sender<ServerEvent>) {
    let mut reader = BufReader::new(stream);
    loop {
        let mut raw = String::new();
        match reader.read_line(&mut raw) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let Ok(event) = decode_server_frame(&raw) else {
                    continue;
                };
                if tx.send(event).is_err() {
                    break;
                }
            }
        }
    }
}
