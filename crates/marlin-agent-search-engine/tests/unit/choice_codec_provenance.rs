// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit tests for Dusa choice-world forced-vs-free provenance summaries.

use super::{format_compact_witness_matrix, summarize_forced_and_free_bindings};
use crate::SearchDslChoiceWorld;

#[test]
fn summarize_forced_and_free_bindings_separates_invariant_from_branching_keys() {
    assert_eq!(summarize_forced_and_free_bindings(&[]), None);

    let worlds = vec![
        SearchDslChoiceWorld {
            world_index: 1,
            bindings: vec![
                vec!["search-dsl/syntax.ss".to_owned(), "rust".to_owned()],
                vec!["ui/help.ss".to_owned(), "scheme".to_owned()],
            ],
            witnesses: Vec::new(),
        },
        SearchDslChoiceWorld {
            world_index: 2,
            bindings: vec![
                vec!["search-dsl/syntax.ss".to_owned(), "rust".to_owned()],
                vec!["ui/help.ss".to_owned(), "rust".to_owned()],
            ],
            witnesses: Vec::new(),
        },
    ];
    let summary = summarize_forced_and_free_bindings(&worlds).expect("non-empty summary");
    assert_eq!(
        summary,
        "forced(1): search-dsl/syntax.ss=rust | free(1): ui/help.ss={rust|scheme}"
    );
    let matrix = format_compact_witness_matrix(&worlds, 16).expect("delta witness matrix");
    assert!(
        matrix.contains("W#1: ui/help.ss=scheme")
            && matrix.contains("W#2: ui/help.ss=rust")
            && !matrix.contains("search-dsl/syntax.ss=rust"),
        "expected delta matrix to list only free keys per world: {matrix}"
    );
}
