// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::TestUiHost;
use marlin_agent_ui::{UiFrameRole, UiOpenFacts};

#[test]
fn linked_scheme_workbench_renders_through_the_native_boundary() {
    let runtime = super::initialize().unwrap();
    let statuses = [
        marlin_agent_ui::UiStatusItem::host(
            marlin_agent_ui::UiComponentId::new("extra/one").unwrap(),
            "鱼",
        ),
        marlin_agent_ui::UiStatusItem::host(
            marlin_agent_ui::UiComponentId::new("extra/two").unwrap(),
            "ready",
        ),
    ];
    let facts = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &statuses,
        extensions: &[],
        method_factories: &[],
    };
    // SAFETY: the runtime and exports originate in the same linked programme.
    let mut host = TestUiHost::from_module(
        unsafe { marlin_agent_ui::UiModule::resolve_linked(&runtime, super::exports()) }.unwrap(),
        &facts,
    )
    .unwrap();
    let candidates = host.command_candidates("mode", None).unwrap().names;
    assert!(!candidates.is_empty());
    assert_eq!(
        host.browse_command_steps(
            "mode",
            &candidates,
            None,
            marlin_agent_ui::UiBrowseDirection::Next,
            33,
            true,
        )
        .unwrap(),
        Some(candidates[32 % candidates.len()].clone())
    );
    // Scheme tests own the workbench tree. This integration check exercises
    // the linked projection and its effect on native terminal geometry.
    let mut app = super::super::app::App::new(marlin_agent_protocol::ModelEndpoint::new(
        "provider", "model",
    ));
    let mut status_terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
    let mut cache = super::super::viewport::TranscriptViewportCache::default();
    let mut anchor = super::super::viewport::ScrollAnchor::default();
    status_terminal
        .draw(|frame| {
            super::super::render::draw(frame, &app, host.view(), &mut cache, &mut anchor);
        })
        .unwrap();
    let row = status_terminal.backend().buffer();
    assert_eq!(row[(3, 11)].symbol(), "鱼");
    assert_eq!(row[(6, 11)].symbol(), "·");
    assert_eq!(row[(8, 11)].symbol(), "r");
    assert_eq!(row[(16, 11)].symbol(), "M");
    app.toggle_command();
    app.command_candidates = vec![marlin_agent_ui::UiCommandName::new("help").unwrap()];
    let regions = super::super::render::frame_regions(
        &app,
        host.view(),
        ratatui::layout::Rect::new(0, 0, 80, 12),
    );
    let roles = &host.view().layout.regions;
    let region = |role| {
        regions
            .iter()
            .find(|region| &region.id == roles.id(role))
            .unwrap()
            .area
    };
    let editor = region(UiFrameRole::Editor);
    let candidates = region(UiFrameRole::Candidates);
    let step = |role| {
        regions
            .iter()
            .find(|region| &region.id == roles.id(role))
            .and_then(|region| region.scroll_step())
    };
    assert_eq!(step(UiFrameRole::Transcript), Some(3));
    assert_eq!(step(UiFrameRole::Candidates), Some(1));
    assert_eq!(editor.height, 1);
    assert_eq!(candidates.height, 1);
    assert_eq!(editor.bottom(), candidates.top());
    assert_eq!(region(UiFrameRole::Modeline).bottom(), editor.top());

    // Candidate count crosses the real Scheme-to-Ratatui measurement boundary.
    let candidates_id = roles.id(UiFrameRole::Candidates);
    for count in [0, 3, 20] {
        app.command_candidates = (0..count)
            .map(|index| marlin_agent_ui::UiCommandName::new(&format!("command-{index}")).unwrap())
            .collect();
        let regions = super::super::render::frame_regions(
            &app,
            host.view(),
            ratatui::layout::Rect::new(0, 0, 80, 40),
        );
        let height = regions
            .iter()
            .find(|region| &region.id == candidates_id)
            .map_or(0, |region| region.area.height);
        assert_eq!(height, count.min(host.view().layout.max_command_rows));
    }

    // Evaluate the linked Scheme list policy across consecutive selections.
    let list = host
        .view()
        .layout
        .frame_tree
        .region_child(candidates_id.as_str())
        .unwrap();
    let mut state = super::super::ui_elements::UiListState::default();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 5)).unwrap();
    for (selected, expected_start) in [(0, 0), (9, 5), (10, 6), (5, 5)] {
        let mut rows = Vec::new();
        terminal
            .draw(|frame| {
                super::super::ui_elements::render_list(
                    frame,
                    frame.area(),
                    list,
                    &host.view().theme,
                    super::super::ui_elements::UiListData {
                        count: 20,
                        selected: Some(selected),
                        extent: &|_| super::super::ui_elements::Extent::Cells(0),
                        facts: &|_| false,
                        bindings: &[],
                    },
                    &mut state,
                    |index| {
                        rows.push(index);
                        Some(super::super::ui_elements::UiListRow {
                            key: None,
                            bindings: vec![],
                        })
                    },
                );
            })
            .unwrap();
        assert_eq!(
            rows,
            (expected_start..expected_start + 5).collect::<Vec<_>>()
        );
    }
    host.refresh_labels("provider/next", "next-session")
        .unwrap();
    assert_eq!(host.view().model_label, "provider/next");
    assert_eq!(host.view().session_label, "next-session");

    // Verify AOT-linked dynamic POO provider descriptor registration over zero-JSON C-ABI.
    let zig_lang = marlin_agent_search_engine::AspLanguageId::new("zig").unwrap();
    let desc_req = marlin_agent_search_engine::DynamicLanguageDescriptorRequest {
        scheme_dir: "",
        language_id: &zig_lang,
        provider_language_id: &zig_lang,
        extensions: &[".zig".to_owned()],
        kind_mappings: &[marlin_agent_search_engine::ForwardedKindMapping {
            native_language_kind: marlin_agent_search_engine::NativeAstKindName::new("comptime_fn")
                .unwrap(),
            general_kind: marlin_agent_search_engine::GeneralOntologyKindName::new("function")
                .unwrap(),
            description: "Zig compile-time generic function".to_owned(),
        }],
        special_specs: &[marlin_agent_search_engine::DynamicSpecialRelationSpec {
            relation_name: "zig_comptime_fn".to_owned(),
            kind_pattern: None,
            language_kind_pattern: Some(
                marlin_agent_search_engine::NativeAstKindName::new("comptime_fn").unwrap(),
            ),
            description: "Zig compile-time generic function".to_owned(),
        }],
    };
    let total_rels = super::register_dynamic_language_descriptor(&runtime, &desc_req).unwrap();
    assert!(total_rels > 20);
    let zig_completions =
        super::search_dsl_completions(&runtime, "(ascent-fragment ((a n) <-- (zig_").unwrap();
    assert!(
        zig_completions.iter().any(|c| c.form == "zig_comptime_fn"),
        "AOT linked-ui completions must advertise dynamically registered zig_comptime_fn"
    );

    // Verify AOT-linked Dusa Scenario B.1 verbatim execution with (scope path-prefix: ...)
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("crates/marlin-agent-search-engine/src/tests")).unwrap();
    std::fs::write(
        ws.join("crates/marlin-agent-search-engine/src/app.rs"),
        "pub fn marlin_search_engine() -> usize { 7 }\n\
         pub fn local_init() -> usize { 1 }\n\
         pub fn run_app() -> usize {\n\
             let a = local_init();\n\
             let b = parse_payload();\n\
             a + b + marlin_search_engine()\n\
         }\n",
    )
    .unwrap();
    std::fs::write(
        ws.join("crates/marlin-agent-search-engine/src/parser_a.rs"),
        "pub fn parse_payload() -> usize { 10 }\n",
    )
    .unwrap();
    std::fs::write(
        ws.join("crates/marlin-agent-search-engine/src/parser_b.rs"),
        "pub fn parse_payload() -> usize { 20 }\n",
    )
    .unwrap();
    std::fs::write(
        ws.join("crates/marlin-agent-search-engine/src/tests/mock_parser.rs"),
        "pub fn parse_payload() -> usize { 99 }\n",
    )
    .unwrap();
    let mut idx = marlin_agent_search_engine::WorkspaceSearchIndex::build_with_config(
        ws,
        marlin_agent_search_engine::WorkspaceIndexConfig::default(),
    );
    idx.ingest_open_edb_facts(marlin_agent_search_engine::WorkspaceOpenEdbIngestBatch {
        language_id: Some("rust".to_owned()),
        provider_id: Some("asp-rust".to_owned()),
        entities: vec![
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/marlin-agent-search-engine/src/app.rs#fn:marlin_search_engine"
                    .to_owned(),
                file_id: 0,
                path: "crates/marlin-agent-search-engine/src/app.rs".to_owned(),
                line: 1,
                name: "marlin_search_engine".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/marlin-agent-search-engine/src/app.rs#fn:local_init".to_owned(),
                file_id: 0,
                path: "crates/marlin-agent-search-engine/src/app.rs".to_owned(),
                line: 2,
                name: "local_init".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/marlin-agent-search-engine/src/app.rs#fn:run_app".to_owned(),
                file_id: 0,
                path: "crates/marlin-agent-search-engine/src/app.rs".to_owned(),
                line: 3,
                name: "run_app".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/marlin-agent-search-engine/src/parser_a.rs#fn:parse_payload"
                    .to_owned(),
                file_id: 0,
                path: "crates/marlin-agent-search-engine/src/parser_a.rs".to_owned(),
                line: 1,
                name: "parse_payload".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id: "crates/marlin-agent-search-engine/src/parser_b.rs#fn:parse_payload"
                    .to_owned(),
                file_id: 0,
                path: "crates/marlin-agent-search-engine/src/parser_b.rs".to_owned(),
                line: 1,
                name: "parse_payload".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            marlin_agent_search_engine::WorkspaceEdbEntityFact {
                entity_id:
                    "crates/marlin-agent-search-engine/src/tests/mock_parser.rs#fn:parse_payload"
                        .to_owned(),
                file_id: 0,
                path: "crates/marlin-agent-search-engine/src/tests/mock_parser.rs".to_owned(),
                line: 1,
                name: "parse_payload".to_owned(),
                language: "rust".to_owned(),
                kind: "function".to_owned(),
                language_kind: "fn_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
        ],
        edges: vec![
            marlin_agent_search_engine::WorkspaceEdbEdgeFact {
                src: "crates/marlin-agent-search-engine/src/app.rs#fn:run_app".to_owned(),
                dst: "local_init".to_owned(),
                relation: "calls".to_owned(),
                path: "crates/marlin-agent-search-engine/src/app.rs".to_owned(),
                line: 4,
            },
            marlin_agent_search_engine::WorkspaceEdbEdgeFact {
                src: "crates/marlin-agent-search-engine/src/app.rs#fn:run_app".to_owned(),
                dst: "parse_payload".to_owned(),
                relation: "calls".to_owned(),
                path: "crates/marlin-agent-search-engine/src/app.rs".to_owned(),
                line: 5,
            },
        ],
        ..Default::default()
    });
    let preset3 = "(search-choice-program (use rust) (scope path-prefix: \"crates/marlin-agent-search-engine/src/\" limit: 12) (choice resolve (callsite target)) (rule (closed (resolve ?call) (?def)) (ref-at ?call ?name) (same-module-def ?call ?name ?def)) (rule (open (resolve ?call) ?def) (ref-at ?call ?name) (workspace-def ?name ?def)) (rule (closed (resolve ?call) ()) (resolve ?call ?def) (in-test-file ?def)) (solve resolve state-budget: 32 provenance?: #t))";
    let res = super::execute_search_dsl_structured(&runtime, &idx, preset3, 16_384).unwrap();
    assert_eq!(
        res.choice_worlds.len(),
        2,
        "rendered:\n{}",
        res.rendered_text
    );
    assert!(!res.rows.is_empty(), "rendered:\n{}", res.rendered_text);

    // Also test with real asp-gateway HTTP hydration path (identical to marlin-tui startup)!
    let asp_bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../agent-semantic-protocols/target/debug/asp-gateway");
    if asp_bin.is_file() {
        let policy = super::helm_index_policy(&runtime).expect("Scheme HelmIndexPolicy.");
        assert_eq!(policy.presets.len(), 5);
        let mut idx2 =
            marlin_agent_search_engine::WorkspaceSearchIndex::build_with_config(ws, policy.index);
        let mut child = std::process::Command::new(&asp_bin)
            .current_dir(ws)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        use std::io::BufRead as _;
        std::io::BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let boot: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let mut ep = marlin_agent_search_engine::AspForwardingEndpoint::new(
            boot["endpoint"].as_str().unwrap(),
        )
        .unwrap();
        if let Some(tok) = boot["controlToken"].as_str() {
            ep = ep.with_control_token(tok);
        }
        let rec = marlin_agent_search_engine::hydrate_workspace_pending_via_asp_http(
            &ep,
            &mut idx2,
            &ws.join(".cache-open-edb.json"),
            &policy.hydration,
        )
        .unwrap();
        assert!(rec.http_ingested_entities > 0);
        let _ = super::sync_asp_provider_descriptors(&runtime, &ep);
        let res2 = super::execute_search_dsl_structured(&runtime, &idx2, preset3, 16_384).unwrap();
        let _ = marlin_agent_search_engine::shutdown_asp_gateway(&ep);
        let _ = child.wait();
        assert_eq!(res2.choice_worlds.len(), 2, "res2:\n{}", res2.rendered_text);
    }
}

#[test]
fn linked_workspace_artifact_layout_projects_default_session_from_scheme() {
    let runtime = super::initialize().unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let artifacts = rt
        .block_on(
            marlin_agent_context::artifacts::WorkspaceArtifacts::discover_with_mode(
                &dir,
                Some(std::ffi::OsStr::new("1")),
            ),
        )
        .expect("discover_with_mode succeeded")
        .expect("dev mode yields Some(WorkspaceArtifacts)");
    let layout = artifacts
        .project_layout(&runtime, |request| {
            Ok(super::artifacts_exchange(&runtime, request)?)
        })
        .expect("linked Scheme artifacts-exchange projects workspace-layout");
    assert_eq!(layout.project, "project:marlin-agent-core");
    assert_eq!(layout.session, "session:default");
    assert_eq!(layout.agent, "agent/host-operator");
}
