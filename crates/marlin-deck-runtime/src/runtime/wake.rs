// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin Unix descriptor ownership for Gambit's native port wait. No scheduler.
use std::{
    io::{self, PipeReader, PipeWriter, Write},
    os::fd::{AsRawFd, FromRawFd, RawFd},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
pub(super) struct Wake {
    // Keep a reader alive through every racing notification, including shutdown.
    reader: PipeReader,
    writer: Mutex<PipeWriter>,
    closed: AtomicBool,
}
impl Wake {
    pub(super) fn new() -> io::Result<Self> {
        let (read, write) = io::pipe()?;
        fn high(fd: RawFd) -> io::Result<RawFd> {
            // SAFETY: dup creates a new, close-on-exec descriptor we own.
            let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
            if duplicate < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(duplicate)
            }
        }
        // SAFETY: each duplicate transfers into exactly one owning pipe handle.
        let reader = unsafe { PipeReader::from_raw_fd(high(read.as_raw_fd())?) };
        let writer = unsafe { PipeWriter::from_raw_fd(high(write.as_raw_fd())?) };
        // SAFETY: the writer is owned here; preserve flags and add nonblocking.
        let flags = unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            reader,
            writer: Mutex::new(writer),
            closed: AtomicBool::new(false),
        })
    }
    pub(super) fn descriptors(&self) -> [u64; 2] {
        [
            self.reader.as_raw_fd() as u64,
            self.writer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_raw_fd() as u64,
        ]
    }
    pub(super) fn notify(&self) -> io::Result<()> {
        if self.closed.load(Ordering::Acquire) {
            return Ok(());
        }
        match self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write_all(&[1])
        {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(()),
            error => error,
        }
    }
    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/wake.rs"]
mod tests;
