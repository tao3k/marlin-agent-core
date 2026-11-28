// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Property-based fuzzing and stress tests for the Helm window, mode tabs,
//! UTF-8/CJK highlight rendering, and Emacs-style `C-g` / `ESC` minibuffer clearing.

use super::{
    HelmOverlay, HelmParsedInput, SharedSearchIndex, helm_window_area, hit_test_helm_mode_tab,
    project_helm_ui_element,
};
use crate::app::App;
use crate::ui_elements::render_overlay_element;
use marlin_agent_protocol::ModelEndpoint;
use marlin_agent_search_engine::{
    HelmSearchPresetSpec, SearchDslCompletionEntry, SearchDslCompletionKind, WorkspaceIndexConfig,
    WorkspaceSearchIndex,
};

fn fixture_presets() -> Vec<HelmSearchPresetSpec> {
    vec![
        HelmSearchPresetSpec {
            shortcut: "1".into(),
            alias: "search".into(),
            tab: "1:Search".into(),
            label: "1/5 Search".into(),
            query: "fn ".into(),
        },
        HelmSearchPresetSpec {
            shortcut: "2".into(),
            alias: "ascent".into(),
            tab: "2:Ascent".into(),
            label: "2/5 Ascent".into(),
            query: "(ascent-fragment ((a n) <-- (callable_def _ _ _ n _)))".into(),
        },
        HelmSearchPresetSpec {
            shortcut: "3".into(),
            alias: "imports".into(),
            tab: "3:Imports".into(),
            label: "3/5 Imports".into(),
            query: "(ascent-fragment ((a s d) <-- (imports_edge s d _ _)))".into(),
        },
        HelmSearchPresetSpec {
            shortcut: "4".into(),
            alias: "dusa".into(),
            tab: "4:Dusa-Resolve".into(),
            label: "4/5 Dusa-Resolve".into(),
            query: "(search-choice-program (scope limit: 8) (choice r (x y)) (solve r state-budget: 16 provenance?: #t))".into(),
        },
        HelmSearchPresetSpec {
            shortcut: "5".into(),
            alias: "tier".into(),
            tab: "5:Dusa-Tier".into(),
            label: "5/5 Dusa-Tier".into(),
            query: "(search-choice-program (scope limit: 8) (choice t (m l)) (solve t state-budget: 16 provenance?: #t))".into(),
        },
    ]
}
use marlin_agent_ui::{UiEditorAction, UiRgb, UiTheme};
use proptest::prelude::{
    Just, ProptestConfig, Strategy, any, prop_assert, prop_assert_eq, prop_oneof, proptest,
};
use std::fs;
use std::sync::{Arc, RwLock};

fn sample_theme() -> UiTheme {
    let rgb = |red, green, blue| UiRgb { red, green, blue };
    UiTheme {
        canvas: rgb(40, 44, 52),
        ink: rgb(187, 194, 207),
        bar: rgb(33, 36, 43),
        accent: rgb(81, 175, 239),
        info: rgb(70, 217, 255),
        success: rgb(152, 190, 101),
        warning: rgb(236, 190, 123),
        muted: rgb(91, 98, 104),
    }
}

fn build_fixture_index() -> (tempfile::TempDir, SharedSearchIndex) {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"fuzz-fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/lib.rs"),
        "// 中文注释测试 fuzzy highlight\npub fn alpha_router() -> usize { 1 }\npub fn parse_payload() -> usize { 2 }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/scheme_sample.ss"),
        ";;; 语义搜索测试\n(def (eval-query x) x)\n",
    )
    .unwrap();
    let idx = WorkspaceSearchIndex::build_with_config(tmp.path(), WorkspaceIndexConfig::default());
    (tmp, Arc::new(RwLock::new(idx)))
}

#[derive(Clone, Debug)]
enum FuzzEvent {
    InsertText(String),
    Backspace,
    ClearInput,
    EmacsEdit(UiEditorAction),
    KillToLineEnd,
    KillToLineStart,
    DeleteBackwardWord,
    Move(isize),
    Tab,
    SelectPreset(usize),
    CyclePreset,
    CyclePresetPrev,
    ToggleMark,
    ToggleMarkAll,
    CycleSource,
    JumpNextSource,
    TogglePreview,
    ClickTab(usize),
    Esc,
    Accept(bool),
    Resize(u16, u16),
}

fn fuzz_event_strategy() -> impl Strategy<Value = FuzzEvent> {
    prop_oneof![
        prop_oneof![
            Just("alpha".to_owned()),
            Just("中文".to_owned()),
            Just("@lang:rust ".to_owned()),
            Just("#local !neg ".to_owned()),
            Just("(ascent-fragment ".to_owned()),
            Just("provenance?: #t))".to_owned()),
            Just("🚀\u{0007}\t\n".to_owned()),
            "[a-zA-Z0-9_#:()!? /.-]{0,16}",
        ]
        .prop_map(FuzzEvent::InsertText),
        Just(FuzzEvent::Backspace),
        Just(FuzzEvent::ClearInput),
        prop_oneof![
            Just(UiEditorAction::LineStart),
            Just(UiEditorAction::LineEnd),
            Just(UiEditorAction::BackwardChar),
            Just(UiEditorAction::ForwardChar),
            Just(UiEditorAction::PreviousWord),
            Just(UiEditorAction::NextWord),
            Just(UiEditorAction::DeleteChar),
            Just(UiEditorAction::PasteSelection),
            Just(UiEditorAction::Undo),
        ]
        .prop_map(FuzzEvent::EmacsEdit),
        Just(FuzzEvent::KillToLineEnd),
        Just(FuzzEvent::KillToLineStart),
        Just(FuzzEvent::DeleteBackwardWord),
        (-10isize..=10isize).prop_map(FuzzEvent::Move),
        Just(FuzzEvent::Tab),
        (0usize..8usize).prop_map(FuzzEvent::SelectPreset),
        Just(FuzzEvent::CyclePreset),
        Just(FuzzEvent::CyclePresetPrev),
        Just(FuzzEvent::ToggleMark),
        Just(FuzzEvent::ToggleMarkAll),
        Just(FuzzEvent::CycleSource),
        Just(FuzzEvent::JumpNextSource),
        Just(FuzzEvent::TogglePreview),
        (0usize..5usize).prop_map(FuzzEvent::ClickTab),
        Just(FuzzEvent::Esc),
        any::<bool>().prop_map(FuzzEvent::Accept),
        (20u16..180u16, 6u16..50u16).prop_map(|(w, h)| FuzzEvent::Resize(w, h)),
    ]
}

fn assert_helm_invariants(helm: &HelmOverlay, width: u16, height: u16) {
    if helm.filtered_indices.is_empty() {
        assert_eq!(helm.selected, 0);
    } else {
        assert!(helm.selected < helm.filtered_indices.len());
    }
    let actions = helm.available_actions();
    if actions.is_empty() {
        assert_eq!(helm.action_selected, 0);
    } else {
        assert!(helm.action_selected < actions.len());
    }
    if helm.completion_entries.is_empty() {
        assert_eq!(helm.completion_selected, 0);
    } else {
        assert!(helm.completion_selected < helm.completion_entries.len());
    }
    // Editor / cursor invariants: `editor` and `input` stay 100% locked in sync
    // and cursor always sits on a valid UTF-8 char boundary.
    assert_eq!(helm.editor.text(), helm.input.as_str());
    assert!(helm.cursor_byte() <= helm.input.len());
    assert!(helm.input.is_char_boundary(helm.cursor_byte()));
    // Clean candidate row invariant: no redundant `[lang·rg]` or `[lang·fuzzy-path...]` noise.
    for cand in &helm.search_candidates {
        assert!(
            !cand.badge.contains("fuzzy-path")
                && !cand.badge.contains("bm25")
                && !cand.badge.contains("·rg"),
            "redundant badge leaked into search candidate: {:?}",
            cand.badge
        );
    }
    // Render under arbitrary terminal geometry without panic!
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
        width.max(1),
        height.max(1),
    ))
    .expect("test backend");
    let theme = sample_theme();
    terminal
        .draw(|frame| {
            if let Some(area) = helm_window_area(frame.area()) {
                let tree = project_helm_ui_element(helm, area);
                render_overlay_element(frame, area, &tree, &theme);
            }
        })
        .expect("render helm overlay");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn helm_parser_never_panics_on_arbitrary_utf8_and_preserves_sexprs(raw in ".*") {
        let parsed = HelmParsedInput::parse(&raw);
        let _ = parsed.highlight_needles();
        if raw.trim_start().starts_with('(') {
            prop_assert!(parsed.local_tokens.is_empty());
            prop_assert_eq!(parsed.async_pattern, raw.trim());
        }
    }

    #[test]
    fn helm_state_machine_fuzz_preserves_bounds_rendering_and_clean_badges(
        events in proptest::collection::vec(fuzz_event_strategy(), 1..80),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let (tmp, idx) = build_fixture_index();
        let mut helm = HelmOverlay::open(
            None,
            tmp.path().to_path_buf(),
            &[],
            WorkspaceIndexConfig::default(),
            Some(idx),
            rt.handle(),
        )
        .with_search_presets(fixture_presets())
        .with_completion_catalog(vec![
            SearchDslCompletionEntry {
                form: "(ascent-fragment ...)".to_owned(),
                kind: SearchDslCompletionKind::Ascent,
                language: "general".to_owned(),
                signature: "(ascent-fragment (rule ...))".to_owned(),
                doc: "Ascent query".to_owned(),
                insert_text: "(ascent-fragment ".to_owned(),
            },
            SearchDslCompletionEntry {
                form: "(search-choice-program ...)".to_owned(),
                kind: SearchDslCompletionKind::Dusa,
                language: "general".to_owned(),
                signature: "(search-choice-program ...)".to_owned(),
                doc: "Dusa solver".to_owned(),
                insert_text: "(search-choice-program ".to_owned(),
            },
        ]);

        let mut width = 120u16;
        let mut height = 30u16;
        let mut yank_register = String::new();
        for event in events {
            match event {
                FuzzEvent::InsertText(text) => helm.insert_text(&text),
                FuzzEvent::Backspace => helm.backspace(),
                FuzzEvent::ClearInput => helm.clear_input(),
                FuzzEvent::EmacsEdit(action) => helm.edit(action, &mut yank_register),
                FuzzEvent::KillToLineEnd => helm.kill_to_line_end(&mut yank_register),
                FuzzEvent::KillToLineStart => helm.kill_to_line_start(&mut yank_register),
                FuzzEvent::DeleteBackwardWord => helm.delete_backward_word(&mut yank_register),
                FuzzEvent::Move(delta) => helm.move_selection(delta),
                FuzzEvent::Tab => helm.toggle_tab_mode(),
                FuzzEvent::SelectPreset(idx) => helm.select_search_preset(idx),
                FuzzEvent::CyclePreset => helm.cycle_search_preset(),
                FuzzEvent::CyclePresetPrev => helm.cycle_search_preset_prev(),
                FuzzEvent::ToggleMark => helm.toggle_mark_and_advance(),
                FuzzEvent::ToggleMarkAll => helm.toggle_mark_all(),
                FuzzEvent::CycleSource => helm.cycle_source_filter(),
                FuzzEvent::JumpNextSource => helm.jump_next_source(),
                FuzzEvent::TogglePreview => helm.toggle_follow_preview(),
                FuzzEvent::ClickTab(tab_idx) => {
                    let full = ratatui::layout::Rect::new(0, 0, width.max(24), height.max(8));
                    if let Some(area) = helm_window_area(full) {
                        // Click near the start of each rendered tab and verify hit-test never panics.
                        let _ = hit_test_helm_mode_tab(
                            area,
                            &helm.search_presets,
                            area.x + 4 + (tab_idx as u16) * 12,
                            area.y + 1,
                        );
                    }
                    helm.select_search_preset(tab_idx);
                }
                FuzzEvent::Esc => {
                    if helm.completion_mode {
                        helm.close_completion_mode();
                    } else if helm.action_mode {
                        helm.toggle_action_mode();
                    }
                }
                FuzzEvent::Accept(export_only) => {
                    let _ = helm.accept(export_only);
                }
                FuzzEvent::Resize(w, h) => {
                    width = w;
                    height = h;
                }
            }
            assert_helm_invariants(&helm, width, height);
        }

        // Unwind invariant: at most 2 ESC steps always exit completion & action sub-modes.
        for _ in 0..2 {
            if helm.completion_mode {
                helm.close_completion_mode();
            } else if helm.action_mode {
                helm.toggle_action_mode();
            }
        }
        prop_assert!(!helm.completion_mode);
        prop_assert!(!helm.action_mode);

        // Clear invariant: `clear_input()` (`C-u`/`C-k`) always empties prompt and resets preset.
        helm.clear_input();
        prop_assert!(helm.input.is_empty());
        prop_assert!(helm.preset_cursor.is_none());
    }

    #[test]
    fn emacs_cancel_always_clears_residual_minibuffer_composer_text(
        chunks in proptest::collection::vec("[^\\x00-\\x1f]{1,40}", 1..8),
    ) {
        let mut app = App::new(ModelEndpoint::new("openai", "gpt-fuzz"));
        for chunk in chunks {
            app.composer.insert_text(&chunk);
        }
        prop_assert!(!app.composer.text().is_empty());
        app.cancel();
        prop_assert!(
            app.composer.text().is_empty(),
            "C-g / Normal ESC must clear residual composer text, found {:?}",
            app.composer.text()
        );
    }
    #[test]
    fn anthropic_thinking_plus_tool_use_continuation_fuzz_accepts_exact_and_rejects_mutations(
        thinking_parts in proptest::collection::vec("[^\\x00-\\x1f]{1,32}", 1..4),
        answer_text in "[^\\x00-\\x1f]{0,40}",
        tool_queries in proptest::collection::vec("[a-zA-Z0-9_() :.-]{1,32}", 0..4),
        include_redacted in any::<bool>(),
    ) {
        use marlin_agent_protocol::{
            ModelContinuation, ModelContinuationKind, ModelToolCall, ModelToolCallId,
            ModelToolName, assistant_gateway_message,
        };
        use marlin_agent_stream::matches_observed_anthropic_thinking;

        let mut blocks = Vec::new();
        let mut full_thinking = String::new();
        for (idx, part) in thinking_parts.iter().enumerate() {
            full_thinking.push_str(part);
            blocks.push(serde_json::json!({
                "type": "thinking",
                "thinking": part,
                "signature": format!("sig-{idx}"),
            }));
        }
        if include_redacted {
            blocks.push(serde_json::json!({
                "type": "redacted_thinking",
                "data": "opaque-redacted-payload",
            }));
        }
        if !answer_text.is_empty() {
            blocks.push(serde_json::json!({
                "type": "text",
                "text": answer_text,
            }));
        }
        let mut tool_calls = Vec::new();
        for (idx, query) in tool_queries.iter().enumerate() {
            let id = format!("call_{idx}");
            let args = serde_json::json!({"query": query});
            blocks.push(serde_json::json!({
                "type": "tool_use",
                "id": id,
                "name": "workspace_search",
                "input": args,
            }));
            tool_calls.push(ModelToolCall {
                id: ModelToolCallId::new(id).unwrap(),
                name: ModelToolName::new("workspace_search").unwrap(),
                arguments: args.as_object().cloned().unwrap(),
            });
        }

        let mut message = assistant_gateway_message(answer_text);
        message.tool_calls = tool_calls;
        message.continuation = Some(ModelContinuation {
            endpoint: ModelEndpoint::new("deepseek", "deepseek-flash"),
            kind: ModelContinuationKind::AnthropicMessages,
            destination: None,
            payload: serde_json::Value::Array(blocks).to_string(),
        });

        prop_assert!(matches_observed_anthropic_thinking(&message, &full_thinking));
        let mut wrong_thinking = full_thinking.clone();
        wrong_thinking.push('!');
        prop_assert!(!matches_observed_anthropic_thinking(&message, &wrong_thinking));
        if !message.tool_calls.is_empty() {
            let mut tampered = message.clone();
            tampered.tool_calls[0].arguments = serde_json::json!({"query": "tampered-query"})
                .as_object()
                .cloned()
                .unwrap();
            prop_assert!(!matches_observed_anthropic_thinking(&tampered, &full_thinking));
        }
    }
}

#[test]
fn helm_inherits_emacs_line_start_end_kill_and_cursor_viewport_scrolling() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let (tmp, idx) = build_fixture_index();
    let mut helm = HelmOverlay::open(
        Some("alpha beta gamma"),
        tmp.path().to_path_buf(),
        &[],
        WorkspaceIndexConfig::default(),
        Some(idx),
        rt.handle(),
    );
    let mut yank = String::new();
    // C-a jumps to start of line; inserting writes at column 0.
    helm.edit(UiEditorAction::LineStart, &mut yank);
    assert_eq!(helm.cursor_byte(), 0);
    helm.insert_text("@lang:rust ");
    assert_eq!(helm.input, "@lang:rust alpha beta gamma");
    // C-e jumps to end of line; inserting appends at the end.
    helm.edit(UiEditorAction::LineEnd, &mut yank);
    assert_eq!(helm.cursor_byte(), helm.input.len());
    helm.insert_text(" #end");
    assert_eq!(helm.input, "@lang:rust alpha beta gamma #end");
    // M-b moves backward a word, C-k kills to line end, C-y yanks it back!
    helm.edit(UiEditorAction::PreviousWord, &mut yank);
    helm.kill_to_line_end(&mut yank);
    assert_eq!(yank, "end");
    assert_eq!(helm.input, "@lang:rust alpha beta gamma #");
    helm.edit(UiEditorAction::PasteSelection, &mut yank);
    assert_eq!(helm.input, "@lang:rust alpha beta gamma #end");
    // On a long 300-char Dusa query, C-a scrolls minibuffer visibly to the start `(search-choice-program`!
    helm.set_input("(search-choice-program (scope limit: 4) (choice tier (mod level)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (solve tier state-budget: 64 provenance?: #t))");
    helm.edit(UiEditorAction::LineStart, &mut yank);
    assert_eq!(helm.cursor_byte(), 0);
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).expect("backend");
    let theme = sample_theme();
    terminal
        .draw(|frame| {
            let area = helm_window_area(frame.area()).expect("area");
            let tree = project_helm_ui_element(&helm, area);
            render_overlay_element(frame, area, &tree, &theme);
        })
        .expect("render");
    let buf = terminal.backend().buffer();
    let bottom: String = (0..80).map(|x| buf[(x, 23)].symbol()).collect();
    assert!(
        bottom.contains("Helm> ▏(search-choice"),
        "expected C-a to scroll Helm minibuffer to beginning of long S-expr, got: {bottom:?}"
    );
}

#[test]
fn helm_top_bar_tab_click_maps_directly_to_every_mode_preset() {
    let presets = fixture_presets();
    let area = helm_window_area(ratatui::layout::Rect::new(0, 0, 120, 30)).expect("area");
    // Rendered top bar: "━━ [1:Search] [2:Ascent] [3:Imports] [4:Dusa-Resolve] [5:Dusa-Tier] ..."
    assert_eq!(
        hit_test_helm_mode_tab(area, &presets, 5, area.y + 1),
        Some(0)
    );
    assert_eq!(
        hit_test_helm_mode_tab(area, &presets, 16, area.y + 1),
        Some(1)
    );
    assert_eq!(
        hit_test_helm_mode_tab(area, &presets, 28, area.y + 1),
        Some(2)
    );
    assert_eq!(
        hit_test_helm_mode_tab(area, &presets, 40, area.y + 1),
        Some(3)
    );
    assert_eq!(
        hit_test_helm_mode_tab(area, &presets, 58, area.y + 1),
        Some(4)
    );
    // Clicking outside the top bar returns None.
    assert_eq!(hit_test_helm_mode_tab(area, &presets, 5, area.y + 2), None);
}

/// Adversarial test refining `SearchQueryMetamorphicAlgebra.qnt` (`stepClippedTabHit` MR6):
/// on a narrow terminal (`width=34`), tabs clipped by `project_top_divider` MUST NOT
/// trigger phantom clicks in `hit_test_helm_mode_tab`.
#[test]
fn helm_hit_test_mode_tab_ignores_tabs_clipped_past_narrow_viewport() {
    let presets = fixture_presets();
    let narrow = helm_window_area(ratatui::layout::Rect::new(0, 0, 34, 14)).expect("narrow area");
    // "━━ " (3) + "[1:Search] " (11) + "[2:Ascent] " (11) = 25 cols used.
    // "[3:Imports]" is 11 cols -> 25 + 11 + 1 = 37 > 34 -> NOT rendered by project_top_divider!
    // Clicking column 28 (1-based) lands inside the unrendered space where tab 2 would have been.
    let hit = hit_test_helm_mode_tab(narrow, &presets, 28, narrow.y + 1);
    assert_eq!(
        hit, None,
        "MR6 CHALLENGE FAILED (clipped tab phantom hit): narrow width=34 returned Some({hit:?}) for an unrendered clipped tab!"
    );
}

/// Adversarial test refining `SearchQueryMetamorphicAlgebra.qnt` (`stepStaleScopeShadow` MR5):
/// applying ScopeDirectory when a `@prefix:` already exists MUST replace the prior prefix
/// (instead of prepending behind last-wins parsing), and MUST NOT corrupt active S-expr prompts.
#[test]
fn helm_scope_directory_replaces_existing_prefix_and_protects_sexpr_prompt() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let (tmp, idx) = build_fixture_index();
    fs::create_dir_all(tmp.path().join("src/nested")).unwrap();
    fs::write(
        tmp.path().join("src/nested/router.rs"),
        "pub fn nested_alpha_router() -> usize { 42 }\n",
    )
    .unwrap();
    {
        let mut guard = idx.write().unwrap();
        let _ = guard.sync_workspace_from_disk();
    }

    let mut helm = HelmOverlay::open(
        Some("@prefix:src/ nested_alpha_router"),
        tmp.path().to_path_buf(),
        &[],
        WorkspaceIndexConfig::default(),
        Some(idx),
        rt.handle(),
    );
    assert!(
        helm.selected_candidate()
            .and_then(|c| c.path.as_deref())
            .is_some_and(|p| p.ends_with("src/nested/router.rs")),
        "expected nested router hit"
    );

    // Open Action Transformer menu and choose ScopeDirectory (`d`).
    helm.toggle_action_mode();
    let dir_idx = helm
        .available_actions()
        .iter()
        .position(|a| a.key == "d")
        .expect("ScopeDirectory action");
    helm.action_selected = dir_idx;
    let (closed, _) = helm.accept(false);
    assert!(!closed);

    assert_eq!(
        helm.parsed.path_prefix.as_deref(),
        Some("src/nested/"),
        "MR5 CHALLENGE FAILED (stale scope shadow): ScopeDirectory produced input {:?} whose parsed path_prefix stayed {:?} instead of Some(\"src/nested/\")!",
        helm.input,
        helm.parsed.path_prefix
    );
}

/// Adversarial test refining `SearchQueryMetamorphicAlgebra.qnt` (MR4 & MR6):
/// repeated '#' stage separators must not inject literal '#' into local_tokens,
/// and non-ASCII `generation_digest` or huge clipboard pastes must never panic or stall.
#[test]
fn helm_parser_multi_hash_resilience_and_utf8_digest_render_safety() {
    let parsed = HelmParsedInput::parse("alpha # beta # !gamma");
    assert!(
        !parsed.local_tokens.iter().any(|t| t == "#"),
        "MR4 CHALLENGE FAILED (multi-# literal leak): local_tokens={:?} still contains literal '#'",
        parsed.local_tokens
    );
    assert_eq!(parsed.local_tokens, vec!["beta", "!gamma"]);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let (tmp, idx) = build_fixture_index();
    let mut helm = HelmOverlay::open(
        Some("alpha"),
        tmp.path().to_path_buf(),
        &[],
        WorkspaceIndexConfig::default(),
        Some(idx),
        rt.handle(),
    );
    // Inject a multi-byte UTF-8 suffix after "blake3-256:" so byte-slicing `&hex[..7]` panics!
    helm.generation_digest = Some("blake3-256:中文摘要测试digest".to_owned());
    assert_helm_invariants(&helm, 100, 24);
}

/// Adversarial test refining `SearchQueryGenerationFencing.qnt` (`stepStaleBatchOverwrite` SM8):
/// stale DSL results carrying an older epoch MUST be dropped rather than overwriting active hits,
/// and giant clipboard pastes MUST be bounded safely on valid UTF-8 boundaries.
#[test]
fn helm_dsl_result_rejects_stale_epoch_and_bounds_clipboard_paste() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let (tmp, idx) = build_fixture_index();
    let mut helm = HelmOverlay::open(
        Some("alpha_router"),
        tmp.path().to_path_buf(),
        &[],
        WorkspaceIndexConfig::default(),
        Some(idx),
        rt.handle(),
    );
    let initial_hits = helm.search_candidates.len();
    assert!(initial_hits > 0);

    // Advance epoch by editing query, then deliver a stale DSL error for the previous epoch.
    let stale_epoch = helm.epoch;
    helm.insert_text("_v2");
    assert!(helm.epoch > stale_epoch);
    let hits_before = helm.search_candidates.len();
    helm.apply_dsl_result(stale_epoch, Err("stale DSL timeout".to_owned()));
    assert_eq!(helm.last_error, None, "stale DSL error must be dropped");
    assert_eq!(helm.search_candidates.len(), hits_before);

    // Giant 10KB multi-byte UTF-8 paste must be bounded cleanly without panic.
    let huge_paste = "中文a".repeat(2_000);
    helm.insert_text(&huge_paste);
    assert!(helm.input.len() <= 2_048);
    assert_helm_invariants(&helm, 100, 24);
}

/// SM16 Adversarial Challenge 1: `UiEditorAction::PasteSelection` (`C-y`) and `set_input`
/// MUST enforce the exact same 2,048-byte ceiling and control-character/ANSI stripping as
/// `insert_text` — pasting a huge multi-line `yank_register` from the main composer MUST NOT
/// bypass `MAX_HELM_INPUT_BYTES` or inject control chars into the single-line minibuffer.
#[test]
fn sm16_paste_selection_and_set_input_enforce_cap_and_strip_controls() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let (tmp, idx) = build_fixture_index();
    let mut helm = HelmOverlay::open(
        Some("alpha"),
        tmp.path().to_path_buf(),
        &[],
        WorkspaceIndexConfig::default(),
        Some(idx),
        rt.handle(),
    );

    // Simulate a 16KB yank_register copied from the multiline composer with newlines, NUL, and ANSI escapes.
    let mut huge_yank = "中文👨‍👩‍👧\n\r\x1b[31m\0abc".repeat(600);
    helm.edit(UiEditorAction::PasteSelection, &mut huge_yank);
    assert!(
        helm.input.len() <= 2_048,
        "SM16 CHALLENGE FAILED (PasteSelection cap bypass): helm.input.len()={} exceeded 2048!",
        helm.input.len()
    );
    assert!(
        !helm.input.chars().any(char::is_control),
        "SM16 CHALLENGE FAILED: control characters leaked via PasteSelection"
    );
    assert_helm_invariants(&helm, 80, 20);

    // Programmatic `set_input` with oversized control-laden text MUST also clamp & strip cleanly.
    let dirty_oversized = format!("foo\n\r\x1b[32mbar{}", "中".repeat(1_500));
    helm.set_input(dirty_oversized);
    assert!(
        helm.input.len() <= 2_048,
        "SM16 CHALLENGE FAILED (set_input cap bypass): helm.input.len()={} exceeded 2048!",
        helm.input.len()
    );
    assert!(
        !helm.input.chars().any(char::is_control),
        "SM16 CHALLENGE FAILED (control injection via set_input): {:?}",
        &helm.input[..helm.input.len().min(40)]
    );
    assert_helm_invariants(&helm, 80, 20);
}

// SM16 Adversarial Challenge 2: stateful `proptest` over `Composer` with Emoji ZWJ,
// combining diacritics, CJK wide chars, narrow widths (0..=6), selections, and undo.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn sm16_composer_grapheme_zwj_combining_narrow_wrap_and_undo_never_panics(
        ops in proptest::collection::vec(0u8..14u8, 1..90),
        widths in proptest::collection::vec(0u16..8u16, 1..20),
    ) {
        let mut composer = crate::composer::Composer::default();
        let corpus = ["e\u{0301}", "👨‍👩‍👧‍👦", "中文搜索", "\n", "(a b)", "🦀\u{200d}🔧", "a\r\nb"];
        for (i, op) in ops.iter().enumerate() {
            let w = widths[i % widths.len()];
            match op {
                0 => composer.insert_text(corpus[i % corpus.len()]),
                1 => composer.insert('中'),
                2 => composer.insert('\u{0301}'),
                3 => composer.left(),
                4 => composer.right(),
                5 => composer.backspace(),
                6 => composer.delete(),
                7 => composer.previous_word(),
                8 => composer.next_word(),
                9 => { let _ = composer.kill_to_line_end(); }
                10 => { let _ = composer.kill_to_line_start(); }
                11 => composer.move_vertical(w, i % 2 == 0),
                12 => composer.select_word(),
                _ => composer.undo(),
            }
            let text = composer.text();
            let cur = composer.cursor();
            prop_assert!(cur <= text.len());
            prop_assert!(text.is_char_boundary(cur));
            let _ = composer.preferred_rows(w, 6);
        }
    }
}
