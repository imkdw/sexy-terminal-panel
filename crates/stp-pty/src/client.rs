use std::fmt::{Debug, Formatter};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use stp_core::protocol::{ClientRequest, ServerEvent, decode_server_frame, encode_client_frame};

use crate::error::BrokerError;

pub struct BrokerClient {
    socket_path: PathBuf,
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl BrokerClient {
    pub fn connect(socket_path: &Path) -> Result<Self, BrokerError> {
        let writer = UnixStream::connect(socket_path)?;
        let reader = BufReader::new(writer.try_clone()?);
        Ok(Self {
            socket_path: socket_path.to_path_buf(),
            writer,
            reader,
        })
    }

    pub fn send(&mut self, request: &ClientRequest) -> Result<(), BrokerError> {
        let frame = encode_client_frame(request)?;
        self.writer.write_all(frame.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn request(&mut self, request: &ClientRequest) -> Result<ServerEvent, BrokerError> {
        if let Err(error) = self.send(request) {
            if !is_broken_pipe(&error) {
                return Err(error);
            }
            self.reconnect()?;
            self.send(request)?;
        }
        self.read_event()
    }

    pub fn read_event(&mut self) -> Result<ServerEvent, BrokerError> {
        let mut raw = String::new();
        let bytes = self.reader.read_line(&mut raw)?;
        if bytes == 0 {
            return Err(BrokerError::ConnectionClosed);
        }
        decode_server_frame(&raw).map_err(BrokerError::from)
    }

    pub fn read_event_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<ServerEvent>, BrokerError> {
        self.reader.get_ref().set_read_timeout(Some(timeout))?;
        let result = self.read_event();
        self.reader.get_ref().set_read_timeout(None)?;
        match result {
            Ok(event) => Ok(Some(event)),
            Err(BrokerError::Io(source))
                if source.kind() == std::io::ErrorKind::WouldBlock
                    || source.kind() == std::io::ErrorKind::TimedOut =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn reconnect(&mut self) -> Result<(), BrokerError> {
        let writer = UnixStream::connect(&self.socket_path)?;
        let reader = BufReader::new(writer.try_clone()?);
        self.writer = writer;
        self.reader = reader;
        Ok(())
    }
}

impl Debug for BrokerClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrokerClient").finish_non_exhaustive()
    }
}

fn is_broken_pipe(error: &BrokerError) -> bool {
    matches!(error, BrokerError::Io(source) if source.kind() == std::io::ErrorKind::BrokenPipe)
}
