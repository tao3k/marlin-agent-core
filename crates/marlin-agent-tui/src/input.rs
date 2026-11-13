// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal input normalization, including explicit bracketed paste boundaries.

use std::{
    io::{self, Write},
    sync::atomic::{AtomicI32, Ordering},
};

use termion::event::{Event, Key, MouseButton, MouseEvent};

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
const MAX_PASTE_BYTES: usize = 1024 * 1024;

pub(crate) enum TerminalInput {
    Key(Key),
    MouseMove(u16, u16),
    MouseWheel { up: bool, column: u16, row: u16 },
    WheelWake,
    Paste(String),
    PasteRejected,
}

/// Collapse high-rate trackpad pulses before they enter the bounded key queue.
#[derive(Default)]
pub(crate) struct WheelAccumulator(AtomicI32);

impl WheelAccumulator {
    /// A true result means the event loop needs one wake notification.
    pub(crate) fn push(&self, up: bool) -> bool {
        let step = if up { 1 } else { -1 };
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_add(step))
            })
            .expect("wheel update cannot fail")
            == 0
    }

    /// Bound one frame's scroll while retaining direction and small gestures.
    pub(crate) fn take_lines(&self) -> Option<(bool, usize)> {
        let steps = self.0.swap(0, Ordering::Relaxed);
        (steps != 0).then(|| (steps > 0, steps.unsigned_abs().min(16) as usize * 3))
    }
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
        // Termion 4 classifies SS3 application-cursor arrows and CSI arrows
        // carrying an explicit unmodified modifier as unsupported. Both are
        // emitted by real terminals for the same Up/Down keys.
        if matches!(event, Event::Unsupported(_)) {
            match raw {
                b"\x1bOA" | b"\x1b[1;1A" => return Some(TerminalInput::Key(Key::Up)),
                b"\x1bOB" | b"\x1b[1;1B" => return Some(TerminalInput::Key(Key::Down)),
                _ => {}
            }
        }
        if let Some((column, row)) = mouse_motion(raw) {
            return Some(TerminalInput::MouseMove(column, row));
        }
        match event {
            Event::Key(key) => Some(TerminalInput::Key(key)),
            Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, column, row)) => {
                Some(TerminalInput::MouseWheel {
                    up: true,
                    column,
                    row,
                })
            }
            Event::Mouse(MouseEvent::Press(MouseButton::WheelDown, column, row)) => {
                Some(TerminalInput::MouseWheel {
                    up: false,
                    column,
                    row,
                })
            }
            Event::Mouse(
                MouseEvent::Press(_, x, y) | MouseEvent::Hold(x, y) | MouseEvent::Release(x, y),
            ) => Some(TerminalInput::MouseMove(x, y)),
            _ => None,
        }
    }
}

// Termion 4 parses button presses but reports SGR no-button motion as an
// unsupported event. Read only the exact bounded motion sequence it consumed.
fn mouse_motion(raw: &[u8]) -> Option<(u16, u16)> {
    if raw.len() > 24 || !raw.starts_with(b"\x1b[<35;") || !raw.ends_with(b"M") {
        return None;
    }
    let coordinates = std::str::from_utf8(&raw[6..raw.len() - 1]).ok()?;
    let (column, row) = coordinates.split_once(';')?;
    let column = column.parse::<u16>().ok()?;
    let row = row.parse::<u16>().ok()?;
    (column > 0 && row > 0).then_some((column, row))
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

/// SGR any-motion tracking is scoped to the alternate-screen session.
pub(crate) struct MouseMotion<W: Write>(W);

impl<W: Write> MouseMotion<W> {
    pub(crate) fn enable(mut writer: W) -> io::Result<Self> {
        writer.write_all(b"\x1b[?1003h\x1b[?1006h")?;
        writer.flush()?;
        Ok(Self(writer))
    }
}

impl<W: Write> Write for MouseMotion<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<W: Write> Drop for MouseMotion<W> {
    fn drop(&mut self) {
        let _ = self.0.write_all(b"\x1b[?1006l\x1b[?1003l");
        let _ = self.0.flush();
    }
}
