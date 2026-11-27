// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Single owner for the terminal's raw, alternate-screen, paste and mouse modes.

use std::io::{self, BufWriter, Stdout, Write};

use ratatui::{Terminal, backend::TermionBackend};
use termion::{
    raw::{IntoRawMode, RawTerminal},
    screen::{AlternateScreen, IntoAlternateScreen},
};

use super::{BracketedPaste, MouseMotion};

const FRAME_BUFFER_BYTES: usize = 64 * 1024;
const SYNC_UPDATE_BEGIN: &[u8] = b"\x1b[?2026h";
const SYNC_UPDATE_END: &[u8] = b"\x1b[?2026l";

type RawOutput = BracketedPaste<MouseMotion<AlternateScreen<RawTerminal<Stdout>>>>;
type NativeTerminal = Terminal<TermionBackend<BufWriter<RawOutput>>>;

pub(crate) struct TerminalOwner {
    active: Option<NativeTerminal>,
}

impl TerminalOwner {
    pub(crate) fn open() -> io::Result<Self> {
        Ok(Self {
            active: Some(open_terminal()?),
        })
    }

    /// Render one frame atomically inside a DEC Mode 2026 Synchronized Update
    /// bracket backed by a 64 KiB `BufWriter`, letting GPU terminals (Ghostty,
    /// Kitty, WezTerm) composite the full diff in a single vsync without tearing.
    pub(crate) fn draw_synchronized(
        &mut self,
        render: impl FnOnce(&mut ratatui::Frame<'_>),
    ) -> io::Result<()> {
        let active = self.active.as_mut().expect("terminal is active");
        active.backend_mut().write_all(SYNC_UPDATE_BEGIN)?;
        let draw_result = active.draw(render).map(|_| ());
        let end_result = active.backend_mut().write_all(SYNC_UPDATE_END);
        let flush_result = active.backend_mut().flush();
        draw_result?;
        end_result?;
        flush_result
    }

    pub(crate) fn suspend(&mut self) -> io::Result<()> {
        let Some(mut active) = self.active.take() else {
            return Ok(());
        };
        let cursor = active.show_cursor();
        let _ = active.backend_mut().flush();
        drop(active);
        // termion's alternate-screen guard writes its exit on Drop but does
        // not flush stdout. The shell must see it before the process stops.
        let flushed = io::stdout().flush();
        cursor?;
        flushed
    }

    pub(crate) fn resume(&mut self) -> io::Result<()> {
        if self.active.is_none() {
            self.active = Some(open_terminal()?);
        }
        Ok(())
    }
}

impl Drop for TerminalOwner {
    fn drop(&mut self) {
        if let Some(mut active) = self.active.take() {
            let _ = active.show_cursor();
            let _ = active.backend_mut().flush();
            drop(active);
            let _ = io::stdout().flush();
        }
    }
}

fn open_terminal() -> io::Result<NativeTerminal> {
    let result = (|| {
        let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
        let stdout = BracketedPaste::enable(MouseMotion::enable(stdout)?)?;
        let buffered = BufWriter::with_capacity(FRAME_BUFFER_BYTES, stdout);
        Terminal::new(TermionBackend::new(buffered))
    })();
    if result.is_err() {
        let _ = io::stdout().flush();
    }
    result
}
