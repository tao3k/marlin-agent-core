use super::{composer::Composer, prompt_history::PromptHistory};
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
