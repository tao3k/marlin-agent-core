// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    composer::Composer,
    prompt_history::{HistorySearch, PromptHistory},
};
use marlin_agent_protocol::{assistant_gateway_message, user_gateway_message};

#[test]
fn browsing_restores_original_draft_and_cursor_and_skips_non_user_messages() {
    let messages = vec![
        user_gateway_message("first 中"),
        assistant_gateway_message("one"),
        user_gateway_message("second\nline"),
        assistant_gateway_message("two"),
    ];
    let mut draft = Composer::default();
    draft.insert_text("draft🐳!");
    draft.left();
    let mut history = PromptHistory::default();
    history.previous(&mut draft, &messages);
    assert_eq!(draft.text(), "second\nline");
    draft.insert_text(" edit");
    history.previous(&mut draft, &messages);
    assert_eq!(draft.text(), "first 中");
    history.previous(&mut draft, &messages);
    assert_eq!(draft.text(), "first 中");
    history.next(&mut draft, &messages);
    assert_eq!(draft.text(), "second\nline");
    history.next(&mut draft, &messages);
    assert_eq!(draft.text(), "draft🐳!");
    draft.insert_text("X");
    assert_eq!(draft.text(), "draft🐳X!");
}

#[test]
fn oversized_history_entry_is_skipped_without_replacing_draft() {
    let messages = vec![
        user_gateway_message("ok"),
        user_gateway_message("x".repeat(1024 * 1024 + 1)),
    ];
    let mut draft = Composer::default();
    draft.insert_text("current");
    let mut history = PromptHistory::default();
    history.previous(&mut draft, &messages);
    assert_eq!(draft.text(), "ok");
    history.next(&mut draft, &messages);
    assert_eq!(draft.text(), "current");
}

#[test]
fn search_filters_user_prompts_cycles_and_edits_unicode_query() {
    let messages = vec![
        user_gateway_message("older 🐳"),
        assistant_gateway_message("🐳 ignored"),
        user_gateway_message("newer 🐳"),
        user_gateway_message("unrelated"),
    ];
    let mut search = HistorySearch::default();
    search.insert("🐳", &messages);
    assert_eq!(search.preview().unwrap().text(), "newer 🐳");
    search.previous(&messages);
    assert_eq!(search.preview().unwrap().text(), "older 🐳");
    search.previous(&messages);
    assert_eq!(search.preview().unwrap().text(), "newer 🐳");
    search.insert("missing", &messages);
    assert!(search.preview().is_none());
    for _ in 0..7 {
        search.backspace(&messages);
    }
    assert_eq!(search.query(), "🐳");
    search.backspace(&messages);
    assert_eq!(search.query(), "");
    assert_eq!(search.accept().unwrap().text(), "unrelated");
}

#[test]
fn long_search_is_incremental_and_finds_unicode_across_chunk_boundaries() {
    let content = format!("{}🐳needle{}", "x".repeat(32766), "y".repeat(100_000));
    let history = vec![user_gateway_message(content.clone())];
    let mut search = HistorySearch::default();
    search.insert("🐳needle", &history);
    assert!(search.is_searching());
    assert!(search.preview().is_none());
    while search.is_searching() {
        search.step(&history);
    }
    assert_eq!(search.preview().unwrap().text(), content);

    search.insert("not-present", &history);
    assert!(search.is_searching());
    while search.is_searching() {
        search.step(&history);
    }
    assert!(search.preview().is_none());
}
