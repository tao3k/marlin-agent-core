// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{GitReadError, GitStatusError, MAX_ENTRIES, MAX_OUTPUT_BYTES, parse_status};

#[test]
fn malformed_and_oversized_status_is_rejected_without_partial_results() {
    for value in [
        b"M  path".as_slice(),
        b"R  destination\0".as_slice(),
        b"!! path\0".as_slice(),
        b"M \0".as_slice(),
    ] {
        assert!(matches!(
            parse_status(value),
            Err(GitStatusError::InvalidOutput)
        ));
    }
    let output = b"?? file\0".repeat(MAX_ENTRIES + 1);
    assert!(matches!(
        parse_status(&output),
        Err(GitStatusError::EntryLimit)
    ));
    assert!(matches!(
        parse_status(&vec![0; MAX_OUTPUT_BYTES + 1]),
        Err(GitStatusError::Read(GitReadError::OutputLimit))
    ));
}

#[test]
fn status_parser_preserves_non_utf8_and_escaped_control_bytes() {
    let status = parse_status(b"?? bad\xff\x1bfile\0").unwrap();
    assert_eq!(status.entries[0].path.as_bytes(), b"bad\xff\x1bfile");
    assert_eq!(
        status.entries[0].path.display_escaped(),
        "bad\\xff\\x1bfile"
    );
}
