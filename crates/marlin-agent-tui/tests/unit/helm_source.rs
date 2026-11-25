// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{HelmAction, HelmOverlay, HelmParsedInput, HelmSourceKind};
use crate::helm_render::render_helm_window;
use marlin_agent_search_engine::{WorkspaceIndexConfig, WorkspaceSearchIndex};
use marlin_agent_ui::{UiRgb, UiTheme};
use std::path::Path;
use std::sync::Arc;

fn scheme_like_config() -> WorkspaceIndexConfig {
    WorkspaceIndexConfig {
        skip_dirs: vec![
            "target".into(),
            ".git".into(),
            ".gerbil".into(),
            "node_modules".into(),
            ".devenv".into(),
            ".direnv".into(),
            "dist".into(),
            "build".into(),
        ],
        max_file_bytes: 2 * 1024 * 1024,
        max_line_matches_per_hit: 6,
        max_line_preview_chars: 220,
        fuzzy_distance: 1,
        enable_tantivy: true,
        enable_file_locator: true,
        enable_rg_regex: true,
    }
}

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

#[test]
fn two_stage_consult_orderless_parser_separates_async_and_local_tokens() {
    let parsed = HelmParsedInput::parse(
        "@lang:rust @doc:org @prefix:crates/ @ext:rs execute_admitted #storage !test",
    );
    assert_eq!(parsed.async_pattern, "execute_admitted");
    assert_eq!(parsed.local_tokens, vec!["storage", "!test"]);
    assert_eq!(parsed.languages, vec!["rust"]);
    assert_eq!(parsed.documents, vec!["org"]);
    assert_eq!(parsed.path_prefix.as_deref(), Some("crates/"));
    assert_eq!(parsed.extension.as_deref(), Some("rs"));
}

#[test]
fn warm_trigram_helm_executes_instantly_and_supports_emacs_source_jump_mark_and_actions() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let cfg = scheme_like_config();
    let warm_index = Arc::new(WorkspaceSearchIndex::build_with_config(
        &workspace_root,
        cfg.clone(),
    ));
    assert!(warm_index.file_count() > 50);

    let mut overlay = HelmOverlay::open(
        Some("@lang:rust @prefix:crates/marlin-agent-search-engine/src/ marlin_search_engine"),
        workspace_root,
        &[],
        cfg,
        Some(warm_index),
        rt.handle(),
    );
    assert!(!overlay.is_indexing());
    assert!(overlay.generation_digest.is_some());
    assert!(overlay.last_query_micros.is_some());
    eprintln!(
        "warm Helm trigram query elapsed: {} ({} indexed files)",
        overlay.format_latency(),
        overlay.indexed_files
    );

    assert!(
        overlay.search_candidates.iter().any(|cand| cand
            .display
            .contains("crates/marlin-agent-search-engine/src/engine.rs")),
        "expected engine.rs hit, got {:?}",
        overlay.search_candidates
    );
    // Surrounding context lines are pre-extracted directly from RAM for follow-preview!
    assert!(
        overlay
            .selected_candidate()
            .is_some_and(|cand| cand.context_lines.len() >= 3)
    );

    // Local Orderless filter must NOT bump epoch.
    let epoch_before = overlay.epoch;
    overlay.insert_text(" #engine !storage");
    assert_eq!(overlay.epoch, epoch_before);
    assert!(!overlay.filtered_indices.is_empty());

    // C-o jumps across source groups cleanly (Search -> Buffers if present, or wraps).
    overlay.insert_text("");
    overlay.cycle_source_filter();
    assert_eq!(overlay.active_source, HelmSourceKind::Search);

    // C-SPC mark-and-advance + TAB Action Transformer mode + export citations.
    overlay.toggle_mark_and_advance();
    assert_eq!(overlay.marked_ids.len(), 1);
    overlay.toggle_action_mode();
    assert!(overlay.action_mode);
    assert!(!overlay.available_actions().is_empty());

    let (should_close, action) = overlay.accept(true);
    assert!(should_close);
    match action {
        Some(HelmAction::InsertCitations(citations)) => {
            assert!(
                citations.contains("@crates/marlin-agent-search-engine/src/engine.rs:"),
                "unexpected citations: {citations}"
            );
        }
        other => panic!("expected InsertCitations, got {other:?}"),
    }
}

#[test]
fn emacs_bottom_split_helm_window_renders_source_headers_preview_modeline_and_minibuffer() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let cfg = scheme_like_config();
    let warm_index = Arc::new(WorkspaceSearchIndex::build_with_config(
        &workspace_root,
        cfg.clone(),
    ));
    let mut overlay = HelmOverlay::open(
        Some("@lang:rust @prefix:crates/marlin-agent-search-engine/src/ marlin_search_engine"),
        workspace_root,
        &[],
        cfg,
        Some(warm_index),
        rt.handle(),
    );

    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30)).expect("test backend");
    let theme = sample_theme();
    terminal
        .draw(|frame| {
            render_helm_window(&overlay, frame, frame.area(), &theme);
        })
        .expect("render bottom-split Helm window");

    let buf = terminal.backend().buffer();
    let rows: Vec<String> = (0..30)
        .map(|y| {
            (0..120)
                .map(|x| buf[(x, y)].symbol())
                .collect::<Vec<_>>()
                .join("")
        })
        .collect();
    // Top rows stay unobstructed so the main conversation buffer remains visible above Helm!
    assert!(rows[0].trim().is_empty());
    // Bottom-most row is the authentic Emacs Helm minibuffer prompt.
    assert!(
        rows[29].contains("Helm>"),
        "expected Helm> minibuffer on bottom row, got: {:?}",
        rows[29]
    );
    // Second-to-last row is the authentic Doom-style Helm modeline.
    assert!(
        rows[28].contains("HELM"),
        "expected HELM modeline on row 28, got: {:?}",
        rows[28]
    );
    // Source header bar and side-by-side follow preview divider are both rendered.
    let joined = rows.join("\n");
    assert!(
        joined.contains("▌ Workspace Search"),
        "missing source header"
    );
    assert!(
        joined.contains("│"),
        "missing right split follow-preview divider"
    );

    // TAB switches cleanly to the Action Transformer view.
    overlay.toggle_action_mode();
    terminal
        .draw(|frame| {
            render_helm_window(&overlay, frame, frame.area(), &theme);
        })
        .expect("render Helm action transformer");
    let act_buf = terminal.backend().buffer();
    let act_text: String = (0..30)
        .map(|y| {
            (0..120)
                .map(|x| act_buf[(x, y)].symbol())
                .collect::<Vec<_>>()
                .join("")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(act_text.contains("Helm Actions"));
    assert!(act_text.contains("HELM-ACT"));
}

#[test]
fn converged_helm_supports_rg_regex_asp_fuzzy_path_tantivy_bm25_and_typo_fuzzy() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let cfg = scheme_like_config();
    let warm_index = Arc::new(WorkspaceSearchIndex::build_with_config(
        &workspace_root,
        cfg.clone(),
    ));

    // 1. Built-in resident `rg` regex search (`re:...`)
    let rg_overlay = HelmOverlay::open(
        Some("@lang:rust re:pub\\s+fn\\s+marlin_search_engine"),
        workspace_root.clone(),
        &[],
        cfg.clone(),
        Some(warm_index.clone()),
        rt.handle(),
    );
    eprintln!("rg-regex query elapsed: {}", rg_overlay.format_latency());
    assert!(
        rg_overlay.search_candidates.iter().any(|c| c
            .display
            .contains("crates/marlin-agent-search-engine/src/engine.rs")
            && c.badge.contains("rg-regex")),
        "expected rg-regex hit on engine.rs, got {:?}",
        rg_overlay.search_candidates
    );

    // 2. ASP FileLocator + Emacs/fzf subsequence fuzzy path (`helm_render`)
    let path_overlay = HelmOverlay::open(
        Some("hlmrndr"),
        workspace_root.clone(),
        &[],
        cfg.clone(),
        Some(warm_index.clone()),
        rt.handle(),
    );
    eprintln!(
        "fuzzy-path query elapsed: {}",
        path_overlay.format_latency()
    );
    assert!(
        path_overlay
            .search_candidates
            .iter()
            .any(|c| c.display.contains("helm_render.rs")),
        "expected fuzzy subsequence hit on helm_render.rs, got {:?}",
        path_overlay.search_candidates
    );

    // 3. In-RAM Tantivy BM25 multi-term query (`rooted scheme search playbook`)
    let bm25_overlay = HelmOverlay::open(
        Some("@lang:rust rooted scheme search playbook"),
        workspace_root.clone(),
        &[],
        cfg.clone(),
        Some(warm_index.clone()),
        rt.handle(),
    );
    eprintln!(
        "tantivy-bm25 query elapsed: {}",
        bm25_overlay.format_latency()
    );
    assert!(
        bm25_overlay.search_candidates.iter().any(|c| c
            .display
            .contains("crates/marlin-agent-search-engine/src/projection.rs")
            && c.badge.contains("bm25")),
        "expected Tantivy BM25 hit on projection.rs, got {:?}",
        bm25_overlay.search_candidates
    );

    // 4. In-RAM Tantivy `FuzzyTermQuery` typo-tolerant search (`playbok` -> `playbook`)
    let typo_overlay = HelmOverlay::open(
        Some("@lang:rust @prefix:crates/marlin-agent-search-engine/ playbok"),
        workspace_root,
        &[],
        cfg,
        Some(warm_index),
        rt.handle(),
    );
    eprintln!(
        "tantivy-fuzzy typo query elapsed: {}",
        typo_overlay.format_latency()
    );
    assert!(
        !typo_overlay.search_candidates.is_empty()
            && typo_overlay
                .search_candidates
                .iter()
                .any(|c| c.badge.contains("fuzzy")),
        "expected Tantivy FuzzyTermQuery hit for typo `playbok`, got {:?}",
        typo_overlay.search_candidates
    );
}
