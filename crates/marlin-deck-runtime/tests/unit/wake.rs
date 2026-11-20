// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::io::{self, Read, Write};

use super::Wake;

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
