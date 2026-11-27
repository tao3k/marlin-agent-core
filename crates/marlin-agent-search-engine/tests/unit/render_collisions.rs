// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit tests for grouped-by-symbol collision rendering in `edb_bridge::render`.

use super::{is_collision_row_shape, render_grouped_collision_lines};
use crate::SearchDslStructuredRow;

#[test]
fn grouped_collision_renderer_deduplicates_pairs_and_flags_high_cardinality() {
    let rows = vec![
        SearchDslStructuredRow {
            cells: vec![
                "new".to_owned(),
                "crates/a/src/lib.rs".to_owned(),
                "1".to_owned(),
                "crates/b/src/lib.rs".to_owned(),
                "2".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/lib.rs".to_owned()),
            line: Some(1),
        },
        SearchDslStructuredRow {
            cells: vec![
                "Binding".to_owned(),
                "crates/a/src/lib.rs".to_owned(),
                "10".to_owned(),
                "crates/b/src/lib.rs".to_owned(),
                "20".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/lib.rs".to_owned()),
            line: Some(10),
        },
        SearchDslStructuredRow {
            cells: vec![
                "Binding".to_owned(),
                "crates/a/src/lib.rs".to_owned(),
                "10".to_owned(),
                "crates/c/src/lib.rs".to_owned(),
                "30".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/lib.rs".to_owned()),
            line: Some(10),
        },
    ];
    assert!(is_collision_row_shape(&rows));
    let (lines, distinct) = render_grouped_collision_lines(&rows, 4096);
    assert_eq!(distinct, 2);
    assert_eq!(lines.len(), 2);
    assert!(
        lines[0].contains("- Binding [3 file(s)]:")
            && lines[0].contains("crates/a/src/lib.rs:L10")
            && lines[0].contains("crates/b/src/lib.rs:L20")
            && lines[0].contains("crates/c/src/lib.rs:L30")
            && lines[1].contains("- new [2 file(s)]:"),
        "expected domain symbol Binding to rank ahead of boilerplate new: {lines:?}"
    );
}
