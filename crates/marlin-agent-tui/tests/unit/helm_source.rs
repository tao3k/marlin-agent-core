// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    HelmAction, HelmOverlay, HelmParsedInput, HelmSourceKind, SharedSearchIndex, helm_window_area,
    project_helm_ui_element,
};
use crate::ui_elements::render_overlay_element;
use marlin_agent_search_engine::{WorkspaceIndexConfig, WorkspaceSearchIndex};
use marlin_agent_ui::{UiRgb, UiTheme};
use std::path::Path;
use std::sync::{Arc, RwLock};

fn wrap_index(index: WorkspaceSearchIndex) -> SharedSearchIndex {
    Arc::new(RwLock::new(index))
}

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
        "@lang:rust @doc:org @prefix:crates/ @ext:rs @sym:marlin_search_engine @graph:engine execute_admitted #storage !test",
    );
    assert_eq!(parsed.async_pattern, "execute_admitted");
    assert_eq!(parsed.local_tokens, vec!["storage", "!test"]);
    assert_eq!(parsed.languages, vec!["rust"]);
    assert_eq!(parsed.documents, vec!["org"]);
    assert_eq!(parsed.path_prefix.as_deref(), Some("crates/"));
    assert_eq!(parsed.extension.as_deref(), Some("rs"));
    assert_eq!(parsed.symbol.as_deref(), Some("marlin_search_engine"));
    assert_eq!(parsed.graph_target.as_deref(), Some("engine"));
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
    let raw_index = WorkspaceSearchIndex::build_with_config(&workspace_root, cfg.clone());
    assert!(raw_index.file_count() > 50);
    let warm_index = wrap_index(raw_index);

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
    let warm_index = wrap_index(WorkspaceSearchIndex::build_with_config(
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
            let window = helm_window_area(frame.area()).expect("helm window area");
            let tree = project_helm_ui_element(&overlay, window);
            render_overlay_element(frame, window, &tree, &theme);
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
            let window = helm_window_area(frame.area()).expect("helm window area");
            let tree = project_helm_ui_element(&overlay, window);
            render_overlay_element(frame, window, &tree, &theme);
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
    let warm_index = wrap_index(WorkspaceSearchIndex::build_with_config(
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

    // 2. ASP FileLocator + Emacs/fzf subsequence fuzzy path (`helm/view.rs`)
    let path_overlay = HelmOverlay::open(
        Some("hlmvw"),
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
            .any(|c| c.display.contains("helm/view.rs")),
        "expected fuzzy subsequence hit on helm/view.rs, got {:?}",
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

#[test]
fn cached_mmap_warm_start_and_incremental_refresh_detects_disk_edits_and_deletions() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace = tempfile::tempdir().expect("temp workspace");
    let cache_dir = tempfile::tempdir().expect("temp cache");
    let snapshot_path = cache_dir.path().join("helm-test.asp-index.bin");

    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/alpha.rs"),
        "pub fn alpha_initial_marker() -> usize { 1 }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("src/beta.rs"),
        "pub fn beta_removable_marker() -> usize { 2 }\n",
    )
    .unwrap();

    let cfg = scheme_like_config();
    // Cold build creates the mmap binary snapshot on disk.
    let first =
        WorkspaceSearchIndex::open_or_build_cached(workspace.path(), &snapshot_path, cfg.clone());
    assert!(snapshot_path.exists());
    assert_eq!(first.file_count(), 2);

    // Second open loads via zero-copy mmap + disk delta sync.
    let cached = wrap_index(WorkspaceSearchIndex::open_or_build_cached(
        workspace.path(),
        &snapshot_path,
        cfg.clone(),
    ));
    let mut overlay = HelmOverlay::open(
        Some("zebra_refreshed_symbol"),
        workspace.path().to_path_buf(),
        &[],
        cfg,
        Some(cached),
        rt.handle(),
    );
    assert!(
        overlay.search_candidates.is_empty(),
        "unexpected pre-edit hit: {:?}",
        overlay.search_candidates
    );
    let digest_before = overlay.generation_digest.clone();

    // Edit alpha.rs and remove beta.rs on disk, then trigger incremental `C-l` refresh.
    std::fs::write(
        workspace.path().join("src/alpha.rs"),
        "pub fn zebra_refreshed_symbol() -> usize { 42 }\n",
    )
    .unwrap();
    std::fs::remove_file(workspace.path().join("src/beta.rs")).unwrap();

    overlay.refresh_index(rt.handle());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while overlay.poll().is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for incremental Helm index refresh"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(!overlay.is_indexing());
    assert_eq!(overlay.indexed_files, 1);
    assert_ne!(overlay.generation_digest, digest_before);
    assert!(
        overlay
            .search_candidates
            .iter()
            .any(|cand| cand.display.contains("src/alpha.rs")),
        "expected refreshed alpha.rs hit, got {:?}",
        overlay.search_candidates
    );
}

#[test]
fn helm_symbol_and_graph_axes_locate_definitions_callers_and_action_transformer_scopes() {
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
    let warm_index = wrap_index(WorkspaceSearchIndex::build_with_config(
        &workspace_root,
        cfg.clone(),
    ));

    // 1. `@sym:` jumps directly to the exact Rust symbol definition line.
    let mut sym_overlay = HelmOverlay::open(
        Some("@lang:rust @sym:marlin_search_engine"),
        workspace_root.clone(),
        &[],
        cfg.clone(),
        Some(warm_index.clone()),
        rt.handle(),
    );
    assert!(
        sym_overlay.search_candidates.iter().any(|c| c
            .display
            .contains("crates/marlin-agent-search-engine/src/engine.rs:")
            && c.badge.contains("symbol")
            && c.primary_symbol.as_deref() == Some("marlin_search_engine")),
        "expected @sym: hit on engine.rs, got {:?}",
        sym_overlay.search_candidates
    );

    // 2. TAB Action Transformer offers `[s] @sym:` and `[g] @graph:` live scoping actions.
    let actions = sym_overlay.available_actions();
    let graph_action_pos = actions
        .iter()
        .position(|a| a.key == "g")
        .expect("expected [g] graph transformer action");
    sym_overlay.toggle_action_mode();
    sym_overlay.action_selected = graph_action_pos;
    let (should_close, handed_off) = sym_overlay.accept(false);
    assert!(!should_close, "graph scoping keeps Helm open live");
    assert!(handed_off.is_none());
    assert_eq!(sym_overlay.input, "@graph:marlin_search_engine");
    assert!(
        sym_overlay
            .search_candidates
            .iter()
            .any(
                |c| c.path.as_deref() == Some("crates/marlin-agent-search-engine/src/engine.rs")
                    && c.badge.contains("graph")
            ),
        "expected definer in @graph: results, got {:?}",
        sym_overlay.search_candidates
    );
    assert!(
        sym_overlay
            .search_candidates
            .iter()
            .any(|c| c.path.as_deref() == Some("crates/marlin-agent-search-engine/src/lib.rs")),
        "expected importer/re-exporter lib.rs in @graph: results, got {:?}",
        sym_overlay.search_candidates
    );
}

#[test]
fn helm_tab_completion_table_completes_native_ascent_and_poo_relations() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let catalog = vec![
        marlin_agent_search_engine::SearchDslCompletionEntry {
            form: "(ascent-fragment ...)".into(),
            kind: marlin_agent_search_engine::SearchDslCompletionKind::Ascent,
            signature: "(ascent-fragment (relation answer ((path string?))) ...)".into(),
            language: "ascent".into(),
            doc: "Real :gerbil-ascent composable fragment".into(),
            insert_text: "(ascent-fragment (relation answer ((path string?))) ((answer path) <-- (file f \"rust\" path)))".into(),
        },
        marlin_agent_search_engine::SearchDslCompletionEntry {
            form: "rust_fn".into(),
            kind: marlin_agent_search_engine::SearchDslCompletionKind::Relation,
            signature: "(rust_fn c0 c1 c2 c3)".into(),
            language: "rust".into(),
            doc: "Rust function declaration".into(),
            insert_text: "(rust_fn c0 c1 c2 c3)".into(),
        },
    ];

    let mut overlay = HelmOverlay::open(
        Some("("),
        workspace_root,
        &[],
        scheme_like_config(),
        None,
        rt.handle(),
    )
    .with_completion_catalog(catalog);

    // Pressing TAB on `(` opens the structured TAB completion table instead of Action Transformer.
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    assert!(!overlay.action_mode);
    assert_eq!(overlay.completion_entries.len(), 1);
    assert_eq!(overlay.completion_entries[0].form, "(ascent-fragment ...)");

    // Rendering displays the HELM-TAB badge and completion columns cleanly.
    let area = helm_window_area(ratatui::layout::Rect::new(0, 0, 120, 28)).expect("area");
    let element = project_helm_ui_element(&overlay, area);
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 28)).expect("terminal");
    terminal
        .draw(|frame| {
            render_overlay_element(frame, area, &element, &sample_theme());
        })
        .expect("draw");
    let screen: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(screen.contains("HELM-TAB"), "{screen}");
    assert!(screen.contains("ascent-fragment"), "{screen}");

    // Accepting inserts the selected template and stays inside Helm.
    let (should_close, action) = overlay.accept(false);
    assert!(!should_close);
    assert!(action.is_none());
    assert!(!overlay.completion_mode);
    assert!(overlay.input.starts_with("(ascent-fragment "));

    // Live token filtering matches POO-derived `rust_fn` relation.
    overlay.insert_text(" rust_");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    assert_eq!(overlay.completion_entries.len(), 1);
    assert_eq!(overlay.completion_entries[0].form, "rust_fn");
}
