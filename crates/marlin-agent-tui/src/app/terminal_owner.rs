// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Single owner for the terminal's raw, alternate-screen, paste and mouse modes.

use std::io::{self, Stdout, Write};

use ratatui::{Terminal, backend::TermionBackend};
use termion::{
    raw::{IntoRawMode, RawTerminal},
    screen::{AlternateScreen, IntoAlternateScreen},
};

use super::{BracketedPaste, MouseMotion};

type NativeTerminal =
    Terminal<TermionBackend<BracketedPaste<MouseMotion<AlternateScreen<RawTerminal<Stdout>>>>>>;

pub(crate) struct TerminalOwner {
    active: Option<NativeTerminal>,
}

impl TerminalOwner {
    pub(crate) fn open() -> io::Result<Self> {
        Ok(Self {
            active: Some(open_terminal()?),
        })
    }

    pub(crate) fn active(&mut self) -> &mut NativeTerminal {
        self.active.as_mut().expect("terminal is active")
    }

    pub(crate) fn suspend(&mut self) -> io::Result<()> {
        let Some(mut active) = self.active.take() else {
            return Ok(());
        };
        let cursor = active.show_cursor();
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
            drop(active);
            let _ = io::stdout().flush();
        }
    }
}

fn open_terminal() -> io::Result<NativeTerminal> {
    let result = (|| {
        let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
        let stdout = BracketedPaste::enable(MouseMotion::enable(stdout)?)?;
        Terminal::new(TermionBackend::new(stdout))
    })();
    if result.is_err() {
        let _ = io::stdout().flush();
    }
    result
}
