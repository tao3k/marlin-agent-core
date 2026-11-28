// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit tests for grouped-by-symbol collision rendering in `edb_bridge::render`.

use super::{
    is_collision_row_shape, is_impact_row_shape, render_grouped_collision_lines,
    render_grouped_impact_lines, split_identifier_stems,
};
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
    let (lines, distinct) = render_grouped_collision_lines(&rows, None, 4096);
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

    let typed_rows = vec![
        SearchDslStructuredRow {
            cells: vec![
                "struct".to_owned(),
                "GitContext".to_owned(),
                "crates/a/src/repo.rs".to_owned(),
                "40".to_owned(),
                "crates/b/src/git.rs".to_owned(),
                "76".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/repo.rs".to_owned()),
            line: Some(40),
        },
        SearchDslStructuredRow {
            cells: vec![
                "fn".to_owned(),
                "build_graph".to_owned(),
                "crates/a/src/g.rs".to_owned(),
                "12".to_owned(),
                "crates/b/src/g.rs".to_owned(),
                "25".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/g.rs".to_owned()),
            line: Some(12),
        },
    ];
    assert!(is_collision_row_shape(&typed_rows));
    let (typed_lines, typed_distinct) = render_grouped_collision_lines(&typed_rows, None, 4096);
    assert_eq!(typed_distinct, 2);
    let typed_joined = typed_lines.join("");
    assert!(
        typed_joined.contains("[KIND: struct]")
            && typed_joined.contains("GitContext [2 file(s)]:")
            && typed_joined.contains("[KIND: fn]")
            && typed_joined.contains("build_graph [2 file(s)]:"),
        "expected multi-kind grouped collision sections: {typed_joined}"
    );
}

#[test]
fn grouped_impact_renderer_orders_prod_before_test_and_summarizes_roles() {
    let rows = vec![
        SearchDslStructuredRow {
            cells: vec![
                "ref".to_owned(),
                "test".to_owned(),
                "SearchReq".to_owned(),
                "crates/a/tests/t.rs".to_owned(),
                "44".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/tests/t.rs".to_owned()),
            line: Some(44),
        },
        SearchDslStructuredRow {
            cells: vec![
                "def".to_owned(),
                "prod".to_owned(),
                "SearchReq".to_owned(),
                "crates/a/src/types.rs".to_owned(),
                "12".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/types.rs".to_owned()),
            line: Some(12),
        },
        SearchDslStructuredRow {
            cells: vec![
                "caller".to_owned(),
                "prod".to_owned(),
                "run_search".to_owned(),
                "crates/a/src/runner.rs".to_owned(),
                "88".to_owned(),
            ],
            display: String::new(),
            path: Some("crates/a/src/runner.rs".to_owned()),
            line: Some(88),
        },
    ];
    assert!(is_impact_row_shape(&rows));
    let (summary, lines) = render_grouped_impact_lines(&rows, None, 4096);
    assert_eq!(
        summary,
        "prod(2: def=1, caller=1) | test(1: ref=1)\naffected_layers: crates/a(2)"
    );
    let joined = lines.join("");
    let prod_pos = joined.find("[PROD]").expect("PROD section");
    let test_pos = joined.find("[TEST]").expect("TEST section");
    assert!(prod_pos < test_pos, "PROD must appear before TEST: {joined}");
    assert!(
        joined.contains("crates/a/src/types.rs: def[SearchReq@L12]")
            && joined.contains("crates/a/src/runner.rs: caller[run_search@L88]")
            && joined.contains("crates/a/tests/t.rs: ref[L44]"),
        "unexpected grouped impact body: {joined}"
    );
}

#[test]
fn identifier_stem_splitter_handles_pascal_snake_and_kebab_cases() {
    assert_eq!(
        split_identifier_stems("AlphaBetaGamma"),
        vec!["Alpha", "Beta", "Gamma"]
    );
    assert_eq!(
        split_identifier_stems("delta_epsilon_zeta"),
        vec!["delta", "epsilon", "zeta"]
    );
    assert_eq!(
        split_identifier_stems("read-only-gov"),
        vec!["read", "only", "gov"]
    );
}
