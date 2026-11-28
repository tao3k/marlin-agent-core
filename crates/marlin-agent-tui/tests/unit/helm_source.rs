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
            && c.badge.is_empty()),
        "expected clean rg-regex hit on engine.rs, got {:?}",
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
            && c.badge.is_empty()),
        "expected clean Tantivy BM25 hit on projection.rs, got {:?}",
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
                .all(|c| c.badge.is_empty()),
        "expected clean Tantivy FuzzyTermQuery hit for typo `playbok`, got {:?}",
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
            && c.badge.is_empty()
            && c.primary_symbol.as_deref() == Some("marlin_search_engine")),
        "expected clean @sym: hit on engine.rs, got {:?}",
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
                    && c.badge == "defines:marlin_search_engine"
            ),
        "expected definer relation badge in @graph: results, got {:?}",
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

#[test]
fn helm_live_ascent_dsl_mode_projects_rows_ram_preview_repair_badge_and_citations() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/kernel.rs"),
        "// line 1 header\npub fn live_helm_dsl_kernel() -> usize { 99 }\n// line 3 footer\n",
    )
    .unwrap();

    let cfg = scheme_like_config();
    let raw_index = WorkspaceSearchIndex::build_with_config(workspace.path(), cfg.clone());
    let generation = raw_index.generation_digest().as_str().to_owned();
    let warm_index = wrap_index(raw_index);

    let mock_runner: marlin_agent_search_engine::SearchDslStructuredRunner =
        Arc::new(move |_idx, _query, _limit| {
            Ok(marlin_agent_search_engine::SearchDslStructuredResult {
                generation_digest: generation.clone(),
                elapsed_micros: 140,
                indexed_files: 1,
                matched_files: 1,
                mode: marlin_agent_search_engine::SearchDslExecutionMode::Ascent,
                relation: "answer".to_owned(),
                repairs: vec!["synthesize-head-relation".to_owned()],
                canonical_query: None,
                rows: vec![marlin_agent_search_engine::SearchDslStructuredRow {
                    cells: vec![
                        "src/kernel.rs".to_owned(),
                        "live_helm_dsl_kernel".to_owned(),
                        "2".to_owned(),
                    ],
                    display: "(src/kernel.rs live_helm_dsl_kernel 2)".to_owned(),
                    path: Some("src/kernel.rs".to_owned()),
                    line: Some(2),
                }],
                choice_worlds: Vec::new(),
                budget_exhausted: None,
                evidence_receipt: None,
                rendered_text: "mode: ascent\n".to_owned(),
            })
        });

    let mut overlay = HelmOverlay::open(
        None,
        workspace.path().to_path_buf(),
        &[],
        cfg,
        Some(warm_index),
        rt.handle(),
    )
    .with_dsl_runner(mock_runner);

    overlay.insert_text(
        "((answer path name line) <-- (file f \"rust\" path) (rust_fn _ f name line))",
    );
    assert_eq!(overlay.search_candidates.len(), 1);
    let cand = &overlay.search_candidates[0];
    assert_eq!(cand.source, HelmSourceKind::Dsl);
    assert_eq!(cand.citation.as_deref(), Some("@src/kernel.rs:2"));
    assert!(
        cand.context_lines
            .iter()
            .any(|ctx| ctx.text.contains("live_helm_dsl_kernel") && ctx.is_match),
        "expected RAM follow-preview lines for src/kernel.rs:2, got {:?}",
        cand.context_lines
    );

    // Render and verify HELM-DSL badge, Ascent / Dusa DSL source header, repair badge, and RAM preview.
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
    assert!(screen.contains("HELM-DSL"), "{screen}");
    assert!(screen.contains("Ascent / Dusa DSL"), "{screen}");
    assert!(
        screen.contains("repaired[synthesize-head-relation]"),
        "{screen}"
    );
    assert!(screen.contains("live_helm_dsl_kernel"), "{screen}");

    // Multi-mark + Embark citation export works identically on DSL candidates.
    overlay.toggle_mark_and_advance();
    assert_eq!(overlay.marked_ids.len(), 1);
    let (should_close, action) = overlay.accept(true);
    assert!(should_close);
    assert_eq!(
        action,
        Some(HelmAction::InsertCitations("@src/kernel.rs:2".to_owned()))
    );

    // Main-thread AOT error delivery clears stale candidates and surfaces fail-closed diagnostic.
    let epoch = overlay.epoch;
    overlay.apply_dsl_result(
        epoch,
        Err("unknown :gerbil-ascent relation 'rust_function'; did you mean: rust_fn?".to_owned()),
    );
    assert!(overlay.search_candidates.is_empty());
    assert!(
        overlay
            .last_error
            .as_deref()
            .is_some_and(|e| e.contains("did you mean: rust_fn"))
    );
}

#[test]
fn helm_dusa_multi_world_provenance_preview_and_preset_cycling() {
    use marlin_agent_search_engine::{
        SearchDslChoiceRuleKind, SearchDslChoiceWitness, SearchDslChoiceWitnessKind,
        SearchDslChoiceWorld, SearchDslExecutionMode, SearchDslStructuredResult,
        SearchDslStructuredRow,
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/app.rs"),
        "pub fn local_init() -> usize { 1 }\n\
         pub fn run_app() -> usize {\n\
             let a = local_init();\n\
             let b = parse_payload();\n\
             a + b\n\
         }\n",
    )
    .unwrap();

    let cfg = scheme_like_config();
    let raw_index = WorkspaceSearchIndex::build_with_config(workspace.path(), cfg.clone());
    let generation = raw_index.generation_digest().as_str().to_owned();
    let warm_index = wrap_index(raw_index);

    let mock_runner: marlin_agent_search_engine::SearchDslStructuredRunner = Arc::new(
        move |_idx, _query, _limit| {
            Ok(SearchDslStructuredResult {
                generation_digest: generation.clone(),
                elapsed_micros: 24,
                indexed_files: 4,
                matched_files: 4,
                mode: SearchDslExecutionMode::Choice,
                relation: "resolve".to_owned(),
                repairs: Vec::new(),
                canonical_query: None,
                rows: vec![
                    SearchDslStructuredRow {
                        cells: vec![
                            "src/app.rs#fn:run_app@L3:local_init".to_owned(),
                            "src/app.rs#fn:local_init".to_owned(),
                        ],
                        display: "[W#1] src/app.rs#fn:run_app@L3:local_init -> src/app.rs#fn:local_init  [deduction: closed]".to_owned(),
                        path: Some("src/app.rs".to_owned()),
                        line: Some(3),
                    },
                    SearchDslStructuredRow {
                        cells: vec![
                            "src/app.rs#fn:run_app@L4:parse_payload".to_owned(),
                            "src/parser_a.rs#fn:parse_payload".to_owned(),
                        ],
                        display: "[W#1] src/app.rs#fn:run_app@L4:parse_payload -> src/parser_a.rs#fn:parse_payload  [branch: open]".to_owned(),
                        path: Some("src/app.rs".to_owned()),
                        line: Some(4),
                    },
                ],
                choice_worlds: vec![
                    SearchDslChoiceWorld {
                        world_index: 1,
                        bindings: vec![
                            vec![
                                "src/app.rs#fn:run_app@L3:local_init".to_owned(),
                                "src/app.rs#fn:local_init".to_owned(),
                            ],
                            vec![
                                "src/app.rs#fn:run_app@L4:parse_payload".to_owned(),
                                "src/parser_a.rs#fn:parse_payload".to_owned(),
                            ],
                        ],
                        witnesses: vec![
                            SearchDslChoiceWitness {
                                kind: SearchDslChoiceWitnessKind::Deduction,
                                fact: vec![
                                    "resolve".to_owned(),
                                    "src/app.rs#fn:run_app@L3:local_init".to_owned(),
                                    "src/app.rs#fn:local_init".to_owned(),
                                ],
                                rule_kind: SearchDslChoiceRuleKind::Closed,
                                premises: vec![
                                    "(ref-at src/app.rs#fn:run_app@L3:local_init local_init)"
                                        .to_owned(),
                                ],
                                alternatives: vec!["src/app.rs#fn:local_init".to_owned()],
                            },
                            SearchDslChoiceWitness {
                                kind: SearchDslChoiceWitnessKind::Branch,
                                fact: vec![
                                    "resolve".to_owned(),
                                    "src/app.rs#fn:run_app@L4:parse_payload".to_owned(),
                                    "src/parser_a.rs#fn:parse_payload".to_owned(),
                                ],
                                rule_kind: SearchDslChoiceRuleKind::Open,
                                premises: vec![
                                    "(ref-at src/app.rs#fn:run_app@L4:parse_payload parse_payload)"
                                        .to_owned(),
                                ],
                                alternatives: vec![
                                    "src/parser_a.rs#fn:parse_payload".to_owned(),
                                    "src/parser_b.rs#fn:parse_payload".to_owned(),
                                ],
                            },
                        ],
                    },
                    SearchDslChoiceWorld {
                        world_index: 2,
                        bindings: vec![],
                        witnesses: vec![],
                    },
                ],
                budget_exhausted: None,
                evidence_receipt: None,
                rendered_text: "worlds: 2 (SAT)\n".to_owned(),
            })
        },
    );

    let sample_presets = vec![
        marlin_agent_search_engine::HelmSearchPresetSpec {
            shortcut: "1".into(),
            alias: "search".into(),
            tab: "1:Search".into(),
            label: "1/5 L0: Brute-Force Trigram/BM25".into(),
            query: "fn ".into(),
        },
        marlin_agent_search_engine::HelmSearchPresetSpec {
            shortcut: "2".into(),
            alias: "ascent".into(),
            tab: "2:Ascent".into(),
            label: "2/5 Case A: Polyglot Ontology Callables".into(),
            query: "(ascent-fragment ((answer lang path name line) <-- (file f lang path) (callable_def _ f lang name line)))".into(),
        },
        marlin_agent_search_engine::HelmSearchPresetSpec {
            shortcut: "3".into(),
            alias: "imports".into(),
            tab: "3:Imports".into(),
            label: "3/5 Case A: Imports Graph".into(),
            query: "(ascent-fragment ((answer src dst path line) <-- (imports_edge src dst path line)))".into(),
        },
        marlin_agent_search_engine::HelmSearchPresetSpec {
            shortcut: "4".into(),
            alias: "dusa".into(),
            tab: "4:Dusa-Resolve".into(),
            label: "4/5 Case B.1: Dusa Call-Site Disambiguation".into(),
            query: "(search-choice-program (scope limit: 12) (choice resolve (callsite target)) (rule (closed (resolve ?call) (?def)) (ref-at ?call ?name) (same-module-def ?call ?name ?def)) (solve resolve state-budget: 32 provenance?: #t))".into(),
        },
        marlin_agent_search_engine::HelmSearchPresetSpec {
            shortcut: "5".into(),
            alias: "tier".into(),
            tab: "5:Dusa-Tier".into(),
            label: "5/5 Case B.2: Dusa Architecture Tier".into(),
            query: "(search-choice-program (scope limit: 12) (choice tier (mod level)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (solve tier state-budget: 64 provenance?: #t))".into(),
        },
    ];

    let mut overlay = HelmOverlay::open(
        None,
        workspace.path().to_path_buf(),
        &[],
        cfg,
        Some(warm_index),
        rt.handle(),
    )
    .with_search_presets(sample_presets)
    .with_dsl_runner(mock_runner)
    .with_asp_status_badge("ASP:5p·153e".to_owned());

    // Cycling Ctrl-t presets walks L0 -> Case A -> Case A Imports -> Case B.1 Dusa -> Case B.2 Dusa!
    overlay.cycle_search_preset();
    assert!(
        overlay
            .active_preset_label
            .as_deref()
            .unwrap()
            .contains("L0")
    );
    overlay.cycle_search_preset();
    assert!(
        overlay
            .active_preset_label
            .as_deref()
            .unwrap()
            .contains("Case A")
    );
    overlay.cycle_search_preset();
    overlay.cycle_search_preset();
    assert!(
        overlay
            .active_preset_label
            .as_deref()
            .unwrap()
            .contains("Case B.1")
    );
    assert!(overlay.input.starts_with("(search-choice-program "));
    assert_eq!(overlay.last_choice_worlds, 2);
    assert_eq!(overlay.search_candidates[0].badge, "W#1·deduction");
    assert_eq!(overlay.search_candidates[1].badge, "W#1·branch");
    assert!(
        overlay.search_candidates[0]
            .context_lines
            .iter()
            .any(|l| l.text.contains("Dusa World #1 [deduction · closed rule]"))
    );

    let area = helm_window_area(ratatui::layout::Rect::new(0, 0, 128, 30)).expect("area");
    let element = project_helm_ui_element(&overlay, area);
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(128, 30)).expect("terminal");
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
    assert!(screen.contains("HELM-DUSA"), "{screen}");
    assert!(screen.contains("ASP:5p·153e"), "{screen}");
    assert!(screen.contains("2 worlds (SAT)"), "{screen}");
    assert!(screen.contains("W#1·deduction"), "{screen}");
    assert!(screen.contains("Dusa World #1 [deduction"), "{screen}");
    assert!(screen.contains("[4:Dusa-Resolve]"), "{screen}");
    assert!(screen.contains("M-1..5:Mode"), "{screen}");
}

#[test]
fn helm_copilot_context_aware_keyword_and_rust_value_completion_chains_and_renders_ghost_text() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::create_dir_all(workspace.path().join("crates/demo/src")).unwrap();
    std::fs::write(
        workspace.path().join("crates/demo/src/engine.rs"),
        "pub struct SearchEngine;\npub fn run_engine() {}\n",
    )
    .unwrap();
    let mut raw_index =
        WorkspaceSearchIndex::build_with_config(workspace.path(), scheme_like_config());
    raw_index.ingest_open_edb_facts(marlin_agent_search_engine::WorkspaceOpenEdbIngestBatch {
        schema_id: None,
        schema_version: None,
        language_id: Some("rust".to_owned()),
        provider_id: Some("test-provider".to_owned()),
        entities: vec![
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/demo/src/engine.rs#struct:SearchEngine".to_owned(),
                file_id: 0,
                path: "crates/demo/src/engine.rs".to_owned(),
                line: 1,
                name: "SearchEngine".to_owned(),
                language: "rust".to_owned(),
                kind: "struct".to_owned(),
                language_kind: "struct_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/demo/src/engine.rs#fn:run_engine".to_owned(),
                file_id: 0,
                path: "crates/demo/src/engine.rs".to_owned(),
                line: 2,
                name: "run_engine".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
        ],
        edges: Vec::new(),
    });
    let warm_index = wrap_index(raw_index);

    // Build completion catalog 100% dynamically from ASP's `GET /v1/providers`
    // (`ForwardedProviderCatalog`: `universalProtocolSchema` + per-language `manifest.project`
    // + `manifest.special` + composed `targetedSchema`) with ZERO hardcoding in Marlin!
    let provider_catalog: marlin_agent_search_engine::ForwardedProviderCatalog =
        serde_json::from_value(serde_json::json!({
            "schemaId": "agent.semantic-protocols.provider-forwarding-catalog",
            "schemaVersion": "1",
            "generation": 1,
            "registryDigest": "blake3-256:0000000000000000000000000000000000000000000000000000000000000000",
            "universalProtocolSchema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "$defs": {
                    "UniversalSearchProjection": {
                        "type": "string",
                        "enum": ["matches", "source", "callable-skeleton"]
                    }
                },
                "properties": {
                    "exclude-tests?": { "type": ["boolean", "null"], "description": "Exclude test files" },
                    "path-prefix": { "type": ["string", "null"], "description": "Workspace-relative path prefix" },
                    "projection": {
                        "anyOf": [
                            { "$ref": "#/$defs/UniversalSearchProjection" },
                            { "type": "null" }
                        ]
                    }
                }
            },
            "providers": [
                {
                    "manifest": {
                        "languageId": "rust",
                        "providerId": "asp-rust",
                        "extensions": [".rs"],
                        "projectMarkers": ["Cargo.toml"],
                        "project": {
                            "markers": ["Cargo.toml"],
                            "entryFiles": ["src/lib.rs", "src/main.rs"],
                            "workspaceFiles": ["Cargo.toml", "Cargo.lock"],
                            "configFiles": ["rustfmt.toml"]
                        },
                        "special": {
                            "kindMappings": [
                                { "nativeLanguageKind": "struct_item", "generalKind": "struct", "description": "Rust struct declaration" },
                                { "nativeLanguageKind": "fn_item", "generalKind": "function", "description": "Rust free function item" }
                            ],
                            "specialRelations": ["rust_fn", "rust_struct"],
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "visibility": { "type": "string", "enum": ["pub", "pub(crate)", "private"] },
                                    "unsafe": { "type": "boolean" }
                                }
                            }
                        }
                    },
                    "targetedSchema": {
                        "type": "object",
                        "properties": {
                            "kind": { "type": "string", "enum": ["struct", "function", "trait"] },
                            "languageKind": { "type": "string", "enum": ["struct_item", "fn_item", "impl_item"] },
                            "visibility": { "type": "string", "enum": ["pub", "pub(crate)", "private"] },
                            "unsafe": { "type": "boolean" }
                        }
                    }
                },
                {
                    "manifest": {
                        "languageId": "zig",
                        "providerId": "asp-zig-dynamic",
                        "extensions": [".zig"],
                        "projectMarkers": ["build.zig"],
                        "project": {
                            "markers": ["build.zig"],
                            "entryFiles": ["src/root.zig", "src/main.zig"]
                        },
                        "special": {
                            "kindMappings": [],
                            "specialRelations": ["zig_comptime"],
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "comptimeMode": { "type": "string", "enum": ["inline", "generic"] }
                                }
                            }
                        }
                    },
                    "targetedSchema": {
                        "type": "object",
                        "properties": {
                            "kind": { "type": "string", "enum": ["function", "struct"] },
                            "comptimeMode": { "type": "string", "enum": ["inline", "generic"] }
                        }
                    }
                }
            ]
        }))
        .expect("forwarded provider catalog JSON");

    let catalog =
        marlin_agent_search_engine::project_provider_catalog_completions(&provider_catalog);

    let mut overlay = HelmOverlay::open(
        Some("(find-collisions lang: \"rust\" "),
        workspace.path().to_path_buf(),
        &[],
        scheme_like_config(),
        Some(warm_index),
        rt.handle(),
    )
    .with_completion_catalog(catalog);

    // 1. After choosing `lang: "rust"`, Copilot suggests ONLY protocol-derived keywords:
    //    Rust's `special` + `targetedSchema` properties (`kind:`, `languageKind:`, `visibility:`, `unsafe:`)
    //    + ASP `universalProtocolSchema` properties (`projection:`, `exclude-tests?:`, `path-prefix:`),
    //    while omitting used `lang:` and Zig-only `comptimeMode:`.
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    let kw_forms: Vec<&str> = overlay.completion_entries.iter().map(|e| e.form.as_str()).collect();
    assert!(kw_forms.contains(&"kind:"), "{kw_forms:?}");
    assert!(kw_forms.contains(&"languageKind:"), "{kw_forms:?}");
    assert!(kw_forms.contains(&"visibility:"), "rust special key must appear: {kw_forms:?}");
    assert!(kw_forms.contains(&"unsafe:"), "rust special key must appear: {kw_forms:?}");
    assert!(kw_forms.contains(&"projection:"), "universal schema key must appear: {kw_forms:?}");
    assert!(kw_forms.contains(&"exclude-tests?:"), "universal nullable bool key must appear: {kw_forms:?}");
    assert!(!kw_forms.contains(&"comptimeMode:"), "zig-only schema key must not leak to rust: {kw_forms:?}");
    assert!(!kw_forms.contains(&"lang:"), "used keyword must be omitted: {kw_forms:?}");

    // 1b. Rust's `special` keyword `visibility:` chains straight into its enum (`"pub"`, `"pub(crate)"`, `"private"`),
    //     and universal nullable `exclude-tests?:` chains into `#t` / `#f`!
    overlay.close_completion_mode();
    overlay.set_input("(find-collisions lang: \"rust\" visibility: ");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    let vis_forms: Vec<&str> = overlay.completion_entries.iter().map(|e| e.form.as_str()).collect();
    assert!(vis_forms.contains(&"\"pub\"") && vis_forms.contains(&"\"pub(crate)\""), "{vis_forms:?}");
    overlay.close_completion_mode();
    overlay.set_input("(find-collisions lang: \"rust\" exclude-tests?: ");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    let bool_forms: Vec<&str> = overlay.completion_entries.iter().map(|e| e.form.as_str()).collect();
    assert!(bool_forms.contains(&"#t") && bool_forms.contains(&"#f"), "{bool_forms:?}");
    overlay.close_completion_mode();
    overlay.set_input("(find-collisions lang: \"rust\" ");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);

    // 2. Accepting `kind:` inserts `kind: ` AND auto-chains into Rust's JSON-Schema `kind` enum!
    let (should_close, action) = overlay.accept(false);
    assert!(!should_close);
    assert!(action.is_none());
    assert_eq!(overlay.input, "(find-collisions lang: \"rust\" kind: ");
    assert!(
        overlay.completion_mode,
        "accepting a keyword should chain directly into its JSON-Schema value completions"
    );
    let val_forms: Vec<&str> = overlay.completion_entries.iter().map(|e| e.form.as_str()).collect();
    assert!(val_forms.contains(&"\"struct\""), "{val_forms:?}");
    assert!(val_forms.contains(&"\"trait\""), "{val_forms:?}");

    // 3. Typing partial `"st` filters to `"struct"` with ghost suffix `ruct"`,
    //    and accepting replaces `"st` cleanly with `"struct" ` (no doubled quotes).
    overlay.insert_text("\"st");
    assert_eq!(overlay.copilot_ghost_suffix().as_deref(), Some("ruct\""));
    let _ = overlay.accept(false);
    assert_eq!(overlay.input, "(find-collisions lang: \"rust\" kind: \"struct\" ");
    assert!(!overlay.completion_mode);

    // 4. Switching language to dynamically forwarded `zig` exposes Zig's custom JSON-Schema
    //    property `comptimeMode:` and enum values (`"inline"`, `"generic"`) — ZERO hardcoding!
    overlay.set_input("(find-symbols lang: \"zig\" ");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    assert!(
        overlay.completion_entries.iter().any(|e| e.form == "comptimeMode:"),
        "{:?}",
        overlay.completion_entries
    );
    overlay.close_completion_mode();
    overlay.set_input("(find-symbols lang: \"zig\" comptimeMode: ");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    let zig_vals: Vec<&str> = overlay.completion_entries.iter().map(|e| e.form.as_str()).collect();
    assert!(zig_vals.contains(&"\"inline\"") && zig_vals.contains(&"\"generic\""), "{zig_vals:?}");

    // 5. `(read-file "<path>" symbol: ` surfaces live symbols from WorkspaceSearchIndex!
    overlay.close_completion_mode();
    overlay.set_input("(read-file \"crates/demo/src/engine.rs\" symbol: ");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    let sym_forms: Vec<&str> = overlay.completion_entries.iter().map(|e| e.form.as_str()).collect();
    assert!(sym_forms.contains(&"\"SearchEngine\""), "{sym_forms:?}");
    assert!(sym_forms.contains(&"\"run_engine\""), "{sym_forms:?}");

    // 6. Plain human Helm `@lang:zi` and `@ext:` facets complete from forwarded provider manifests!
    overlay.close_completion_mode();
    overlay.set_input("engine @lang:zi");
    overlay.toggle_tab_mode();
    assert!(overlay.completion_mode);
    assert_eq!(overlay.completion_entries[0].form, "@lang:zig");
    let _ = overlay.accept(false);
    assert_eq!(overlay.input, "engine @lang:zig ");
}
