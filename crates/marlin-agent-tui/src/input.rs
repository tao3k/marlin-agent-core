//! Terminal input normalization, including explicit bracketed paste boundaries.

use std::io::{self, Write};

use termion::event::{Event, Key};

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
const MAX_PASTE_BYTES: usize = 1024 * 1024;

pub(crate) enum TerminalInput {
    Key(Key),
    Paste(String),
    PasteRejected,
}

#[derive(Default)]
pub(crate) struct InputDecoder {
    paste: Option<Vec<u8>>,
    overflowed: bool,
}

impl InputDecoder {
    pub(crate) fn accept(&mut self, event: Event, raw: &[u8]) -> Option<TerminalInput> {
        if self.paste.is_some() {
            if raw == PASTE_END {
                let bytes = self.paste.take().expect("paste is active");
                if std::mem::take(&mut self.overflowed) {
                    return Some(TerminalInput::PasteRejected);
                }
                return Some(match String::from_utf8(bytes) {
                    Ok(value) => {
                        TerminalInput::Paste(value.replace("\r\n", "\n").replace('\r', "\n"))
                    }
                    Err(_) => TerminalInput::PasteRejected,
                });
            }
            let paste = self.paste.as_mut().expect("paste is active");
            if raw.len() > MAX_PASTE_BYTES.saturating_sub(paste.len()) {
                self.overflowed = true;
                paste.clear();
            } else if !self.overflowed {
                paste.extend_from_slice(raw);
            }
            return None;
        }
        if raw == PASTE_START {
            self.paste = Some(Vec::new());
            return None;
        }
        match event {
            Event::Key(key) => Some(TerminalInput::Key(key)),
            _ => None,
        }
    }
}

/// Disables bracketed paste when the terminal backend is dropped.
pub(crate) struct BracketedPaste<W: Write>(W);

impl<W: Write> BracketedPaste<W> {
    pub(crate) fn enable(writer: W) -> io::Result<Self> {
        let mut guard = Self(writer);
        guard.write_all(b"\x1b[?2004h")?;
        guard.flush()?;
        Ok(guard)
    }
}

impl<W: Write> Write for BracketedPaste<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<W: Write> Drop for BracketedPaste<W> {
    fn drop(&mut self) {
        let _ = self.0.write_all(b"\x1b[?2004l");
        let _ = self.0.flush();
    }
}
