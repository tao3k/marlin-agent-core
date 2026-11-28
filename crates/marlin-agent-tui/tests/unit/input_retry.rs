// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::RetryInterruptedRead;
use std::collections::VecDeque;
use std::io;
use termion::{event::Event, event::Key, input::TermReadEventsAndRaw};

struct ScriptedReader {
    steps: VecDeque<io::Result<Vec<u8>>>,
}

impl io::Read for ScriptedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(step) = self.steps.pop_front() else {
            return Ok(0);
        };
        let bytes = step?;
        let count = bytes.len().min(buf.len());
        buf[..count].copy_from_slice(&bytes[..count]);
        if count < bytes.len() {
            self.steps.push_front(Ok(bytes[count..].to_vec()));
        }
        Ok(count)
    }
}

#[test]
fn retry_interrupted_read_survives_eintr_before_and_inside_escape_sequences() {
    let reader = ScriptedReader {
        steps: VecDeque::from([
            Err(io::Error::new(io::ErrorKind::Interrupted, "SIGWINCH")),
            Ok(vec![b'\x1b']),
            Err(io::Error::new(io::ErrorKind::Interrupted, "SIGWINCH")),
            Ok(b"[A".to_vec()),
            Err(io::Error::new(io::ErrorKind::Interrupted, "SIGWINCH")),
            Ok(b"x".to_vec()),
        ]),
    };
    let events: Vec<_> = RetryInterruptedRead(reader)
        .events_and_raw()
        .collect::<Result<Vec<_>, _>>()
        .expect("EINTR must be retried transparently");
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].0, Event::Key(Key::Esc));
    assert_eq!(events[1].0, Event::Key(Key::Char('[')));
    assert_eq!(events[2].0, Event::Key(Key::Char('A')));
    assert_eq!(events[3].0, Event::Key(Key::Char('x')));
    // Note: bytes split across separate read() calls parse as individual reads
    // just like termion's 2-byte reader; verify full CSI sequence after EINTR too.
    let csi_reader = ScriptedReader {
        steps: VecDeque::from([
            Err(io::Error::new(io::ErrorKind::Interrupted, "SIGWINCH")),
            Ok(b"\x1b[".to_vec()),
            Err(io::Error::new(io::ErrorKind::Interrupted, "SIGWINCH")),
            Ok(b"B".to_vec()),
        ]),
    };
    let csi_events: Vec<_> = RetryInterruptedRead(csi_reader)
        .events_and_raw()
        .collect::<Result<Vec<_>, _>>()
        .expect("CSI sequence interrupted by EINTR must still decode");
    assert_eq!(csi_events.len(), 1);
    assert_eq!(csi_events[0].0, Event::Key(Key::Down));
}

#[test]
fn retry_interrupted_read_propagates_non_interrupted_errors() {
    let reader = ScriptedReader {
        steps: VecDeque::from([Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "pipe broken",
        ))]),
    };
    let mut iter = RetryInterruptedRead(reader).events_and_raw();
    let err = iter
        .next()
        .expect("error item")
        .expect_err("non-EINTR error must propagate");
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
}
