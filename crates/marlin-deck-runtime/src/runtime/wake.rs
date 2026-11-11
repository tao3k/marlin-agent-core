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
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn saturation_is_a_retained_wake_and_close_stops_publication() {
        let wake = Wake::new().unwrap();
        let [reader, writer] = wake.descriptors();
        assert!(reader >= 3 && writer >= 3);
        // SAFETY: only inspect/set flags on descriptors owned by this test.
        unsafe {
            assert_ne!(
                libc::fcntl(reader as i32, libc::F_GETFD) & libc::FD_CLOEXEC,
                0
            );
            assert_ne!(
                libc::fcntl(writer as i32, libc::F_GETFD) & libc::FD_CLOEXEC,
                0
            );
            assert_ne!(
                libc::fcntl(writer as i32, libc::F_GETFL) & libc::O_NONBLOCK,
                0
            );
            let flags = libc::fcntl(reader as i32, libc::F_GETFL);
            assert_ne!(flags, -1);
            assert_ne!(
                libc::fcntl(reader as i32, libc::F_SETFL, flags | libc::O_NONBLOCK),
                -1
            );
        }
        let mut filled = 0;
        loop {
            let result = wake.writer.lock().unwrap().write(&[1; 4096]);
            match result {
                Ok(count) => {
                    assert!(count > 0);
                    filled += count;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => panic!("unexpected pipe saturation: {result:?}"),
            }
            assert!(filled <= 64 * 1024 * 1024, "unexpected test pipe capacity");
        }
        // A saturated pipe still retains its wake; notify must return promptly.
        wake.notify().unwrap();
        let mut bytes = [0; 4096];
        let mut observed = 0;
        let mut input = &wake.reader;
        loop {
            match input.read(&mut bytes) {
                Ok(count) => {
                    assert!(count > 0);
                    observed += count;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                result => panic!("unexpected wake input: {result:?}"),
            }
        }
        assert!(filled > 0);
        assert_eq!(observed, filled);
        wake.close();
        wake.notify().unwrap();
        assert_eq!(
            input.read(&mut bytes).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
}
