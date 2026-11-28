// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit regressions for L0 pure-Rust RAM primitives: `exclude-path-prefix:`,
//! zero-hit diagnostics, `(file-outline ...)`, and `(read-file ...)`.

#[cfg(feature = "grpc")]
mod l0_grpc_tests {
use std::sync::{Arc, RwLock};

use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCapability, ModelToolDeclaration, ModelToolOutcome, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, ToolDeclarationRequest, ToolParameterName, ToolParameterSpec,
    ToolRuntime,
};
use marlin_agent_search_engine::{
    SearchDslSessionId, SearchDslSessionRunner, SearchPlanProjectionError, WorkspaceEdbEntityFact,
    WorkspaceIndexConfig, WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex,
};
use marlin_agent_tooling::{
    GrpcSearchToolHandler, LoopbackSearchGrpcServer, SearchGatewayService,
    tool_schema_entries_from_declarations,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn l0_exclude_path_prefix_outline_read_file_and_zero_hit_diagnostics() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let files: [(&str, &str); 5] = [
        (
            "crates/core/Cargo.toml",
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
        ),
        (
            "crates/core/src/engine.rs",
            "pub struct SearchEngine {\n    pub active: bool,\n}\n\npub fn run_engine() -> bool {\n    true\n}\n",
        ),
        (
            "crates/core/tests/engine_test.rs",
            "pub struct SearchEngineTestFixture;\nfn test_run_engine() {}\n",
        ),
        (
            "crates/modes/src/lib.rs",
            "pub enum EngineMode {\n    Fast,\n    Safe,\n}\n",
        ),
        (
            "scheme/agent-testing/governance.ss",
            "(def (evaluate-policy req) 'permit)\n(.def ReadOnlyGovernance. write-allowed?: #f)\n",
        ),
    ];
    for (rel, body) in files {
        let full = tmp.path().join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        std::fs::write(&full, body).expect("write fixture");
    }
    let mut index = WorkspaceSearchIndex::build_with_config(
        tmp.path(),
        WorkspaceIndexConfig::default(),
    );
    index.ingest_open_edb_facts(WorkspaceOpenEdbIngestBatch {
        schema_id: None,
        schema_version: None,
        language_id: Some("rust".to_owned()),
        provider_id: Some("test-provider".to_owned()),
        entities: vec![
            WorkspaceEdbEntityFact {
                entity_id: "crates/core/src/engine.rs#struct:SearchEngine".to_owned(),
                file_id: 0,
                path: "crates/core/src/engine.rs".to_owned(),
                line: 1,
                name: "SearchEngine".to_owned(),
                language: "rust".to_owned(),
                kind: "struct".to_owned(),
                language_kind: "struct_item".to_owned(),
                source: "native-parser".to_owned(),
                attrs: Vec::new(),
            },
            WorkspaceEdbEntityFact {
                entity_id: "crates/core/src/engine.rs#fn:run_engine".to_owned(),
                file_id: 0,
                path: "crates/core/src/engine.rs".to_owned(),
                line: 5,
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

    let declaration = ModelToolDeclaration::from_request(ToolDeclarationRequest {
        name: ModelToolName::new("workspace_search").expect("name"),
        description: "search".to_owned(),
        capability: ModelToolCapability::Search,
        parameters: vec![
            ToolParameterSpec::required_string(ToolParameterName::new("query").expect("param"), "")
                .expect("spec"),
        ],
        timeout_seconds: 10,
        output_limit: 16_384,
    })
    .expect("decl");
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(4));
    let noop_dsl: SearchDslSessionRunner = Arc::new(|_, _, _, _| {
        Err(SearchPlanProjectionError::Invalid(
            "L0 test does not invoke Scheme DSL",
        ))
    });
    let service = SearchGatewayService::new(
        Arc::new(RwLock::new(index)),
        tool_schema_entries_from_declarations(std::slice::from_ref(&declaration)),
        noop_dsl,
        owner.context(),
    );
    let server = LoopbackSearchGrpcServer::spawn(service, &owner).expect("spawn loopback gRPC");
    let session_cell = Arc::new(RwLock::new(
        SearchDslSessionId::new("l0-init").expect("session id"),
    ));
    let handler = GrpcSearchToolHandler::new(declaration, server.client(), Arc::clone(&session_cell));

    let run = |id: &str, query: &str| {
        *session_cell.write().expect("session lock") =
            SearchDslSessionId::new(id).expect("session id");
        let call = ModelToolCall {
            id: ModelToolCallId::new(id).expect("call id"),
            name: ModelToolName::new("workspace_search").expect("tool"),
            arguments: serde_json::Map::from_iter([(
                "query".to_owned(),
                serde_json::Value::String(query.to_owned()),
            )]),
        };
        handler.run_tool(call, owner.context())
    };

    // 1. `exclude-path-prefix: "tests/"` isolates production hits cleanly.
    let ModelToolOutcome::Completed(prod_only) = run(
        "c1",
        "(brute-search \"SearchEngine\" path-prefix: \"crates/\" exclude-path-prefix: \"tests/\")",
    )
    .await
    else {
        panic!("expected Completed for exclude-path-prefix");
    };
    assert!(prod_only.contains("crates/core/src/engine.rs"), "{prod_only}");
    assert!(
        !prod_only.contains("crates/core/tests/engine_test.rs"),
        "{prod_only}"
    );
    assert!(prod_only.contains("excluded_files: 1"), "{prod_only}");

    // 2. `(file-outline ...)` returns instant AST symbol map with line numbers.
    let ModelToolOutcome::Completed(outline) =
        run("c2", "(file-outline \"crates/core/src/engine.rs\")").await
    else {
        panic!("expected Completed for file-outline");
    };
    assert!(outline.contains("mode: file-outline"), "{outline}");
    assert!(outline.contains("L1"), "{outline}");
    assert!(outline.contains("SearchEngine"), "{outline}");
    assert!(outline.contains("run_engine"), "{outline}");

    // 3. `(read-file ...)` reads exact line range from RAM without disk I/O.
    let ModelToolOutcome::Completed(slice) = run(
        "c3",
        "(read-file \"crates/core/src/engine.rs\" start: 1 end: 6)",
    )
    .await
    else {
        panic!("expected Completed for read-file");
    };
    assert!(slice.contains("mode: read-file"), "{slice}");
    assert!(slice.contains("L1: pub struct SearchEngine"), "{slice}");
    assert!(slice.contains("L5: pub fn run_engine()"), "{slice}");

    // 4. `(read-file ... around: N context: M)` centers a clean line window around target line.
    let ModelToolOutcome::Completed(around_slice) = run(
        "c3b",
        "(read-file \"crates/core/src/engine.rs\" around: 5 context: 1)",
    )
    .await
    else {
        panic!("expected Completed for read-file around:");
    };
    assert!(around_slice.contains("range: L4..=L6"), "{around_slice}");
    assert!(around_slice.contains("L5: pub fn run_engine()"), "{around_slice}");

    // 5. `(file-outline ... max-per-file: 1)` distributes symbols fairly across files
    //    and skips 0-symbol manifests (`Cargo.toml`) on directory outlines.
    let ModelToolOutcome::Completed(fair_outline) =
        run("c3c", "(file-outline \"crates/\" max-per-file: 1)").await
    else {
        panic!("expected Completed for fair multi-file outline");
    };
    assert!(fair_outline.contains("SearchEngine"), "{fair_outline}");
    assert!(
        !fair_outline.contains("crates/core/Cargo.toml"),
        "0-symbol manifest must be skipped on directory outline: {fair_outline}"
    );
    assert!(fair_outline.contains("omitted_empty_files:"), "{fair_outline}");
    assert!(fair_outline.contains("+1 more in crates/core/src/engine.rs"), "{fair_outline}");

    // 5b. `(file-outline ... kind: "struct")` filters symbol kinds cleanly.
    let ModelToolOutcome::Completed(struct_only) =
        run("c3d", "(file-outline \"crates/\" kind: \"struct\")").await
    else {
        panic!("expected Completed for kind-filtered outline");
    };
    assert!(struct_only.contains("SearchEngine"), "{struct_only}");
    assert!(!struct_only.contains("run_engine"), "{struct_only}");

    // 5c. `(read-file ...)` reports explicit `next_start:` header when truncated.
    let ModelToolOutcome::Completed(paged) = run(
        "c3e",
        "(read-file \"crates/core/src/engine.rs\" start: 1 end: 3)",
    )
    .await
    else {
        panic!("expected Completed for paginated read-file");
    };
    assert!(paged.contains("has_more: true"), "{paged}");
    assert!(paged.contains("next_start: 4"), "{paged}");

    // 6. Zero-hit diagnostic detects R5RS `define` vs Gerbil `def` on `.ss` files!
    let ModelToolOutcome::Completed(scheme_zero) = run(
        "c4",
        "(brute-search \"define\" path-prefix: \"scheme/agent-testing/governance.ss\")",
    )
    .await
    else {
        panic!("expected Completed for scheme zero-hit diagnostic");
    };
    assert!(scheme_zero.contains("hits: 0"), "{scheme_zero}");
    assert!(scheme_zero.contains("Gerbil Scheme"), "{scheme_zero}");

    // 7. Zero-hit regex alternation must NOT emit a false-positive "without re:" claim
    //    when `re:` itself also matches 0 files.
    let ModelToolOutcome::Completed(regex_zero) = run(
        "c5",
        "(brute-search \"NonExistentAlpha|NonExistentBeta\" limit: 20)",
    )
    .await
    else {
        panic!("expected Completed for zero-hit regex alternation");
    };
    assert!(regex_zero.contains("hits: 0"), "{regex_zero}");
    assert!(
        !regex_zero.contains("looks like a regex without"),
        "must not mislead model with false-positive missing-re diagnostic: {regex_zero}"
    );

    // 8. `exclude-tests?: #t` works cleanly on both L0 `brute-search` and `file-outline`.
    let ModelToolOutcome::Completed(brute_exclude_tests) = run(
        "c6",
        "(brute-search \"SearchEngine\" path-prefix: \"crates/\" exclude-tests?: #t)",
    )
    .await
    else {
        panic!("expected Completed for brute-search exclude-tests?");
    };
    assert!(
        brute_exclude_tests.contains("crates/core/src/engine.rs"),
        "{brute_exclude_tests}"
    );
    assert!(
        !brute_exclude_tests.contains("crates/core/tests/engine_test.rs"),
        "{brute_exclude_tests}"
    );
    assert!(
        brute_exclude_tests.contains("exclude_tests=true"),
        "{brute_exclude_tests}"
    );

    let ModelToolOutcome::Completed(outline_exclude_tests) = run(
        "c7",
        "(file-outline \"crates/\" lang: \"any\" kind: \"any\" exclude-tests?: #t)",
    )
    .await
    else {
        panic!("expected Completed for file-outline exclude-tests?");
    };
    assert!(
        outline_exclude_tests.contains("crates/core/src/engine.rs"),
        "{outline_exclude_tests}"
    );
    assert!(
        outline_exclude_tests.contains("SearchEngine")
            && outline_exclude_tests.contains("run_engine"),
        "kind: \"any\" and lang: \"any\" must preserve all production symbols: {outline_exclude_tests}"
    );
    assert!(
        !outline_exclude_tests.contains("crates/core/tests/engine_test.rs"),
        "{outline_exclude_tests}"
    );

    // 9. `(read-file ... symbol: "run_engine" context: 1)` anchors directly to symbol definition.
    let ModelToolOutcome::Completed(sym_read) = run(
        "c8",
        "(read-file \"crates/core/src/engine.rs\" symbol: \"run_engine\" context: 1)",
    )
    .await
    else {
        panic!("expected Completed for symbol-anchored read-file");
    };
    assert!(
        sym_read.contains("symbol_anchor: run_engine @ L5"),
        "{sym_read}"
    );
    assert!(
        sym_read.contains("L5: pub fn run_engine()"),
        "{sym_read}"
    );

    // 10. Zero-hit compound query emits live RAM stem-decomposition hits (`live stem_hits:`).
    let ModelToolOutcome::Completed(stem_diag) = run(
        "c9",
        "(brute-search \"blocked-SearchEngine|SearchEngineMissing\" limit: 20)",
    )
    .await
    else {
        panic!("expected Completed for compound stem diagnostic");
    };
    assert!(stem_diag.contains("hits: 0"), "{stem_diag}");
    assert!(
        stem_diag.contains("live stem_hits:"),
        "expected live stem-decomposition probe on 0-hit compound query: {stem_diag}"
    );
    assert!(
        stem_diag.contains("\"engine\"=") || stem_diag.contains("\"search\"="),
        "expected live stem probe to surface matching RAM stem: {stem_diag}"
    );

    // 11. Exact-file outline truncated by `max-per-file:` emits an exact `next_query:` hint.
    let ModelToolOutcome::Completed(exact_trunc) = run(
        "c10",
        "(file-outline \"crates/core/src/engine.rs\" max-per-file: 1)",
    )
    .await
    else {
        panic!("expected Completed for truncated exact-file outline");
    };
    assert!(
        exact_trunc.contains("next_query: (file-outline \"crates/core/src/engine.rs\" limit: 2)"),
        "expected actionable exact-file next_query hint: {exact_trunc}"
    );

    // 12. Multi-kind pipe filter `kind: "struct|fn"` matches both structs and functions in one call.
    let ModelToolOutcome::Completed(multi_kind) = run(
        "c11",
        "(file-outline \"crates/core/src/engine.rs\" kind: \"struct|fn\")",
    )
    .await
    else {
        panic!("expected Completed for pipe-separated multi-kind outline");
    };
    assert!(
        multi_kind.contains("SearchEngine") && multi_kind.contains("run_engine"),
        "expected pipe-separated multi-kind filter to return both symbols: {multi_kind}"
    );

    // 13. Zero-hit kind-filtered outline reports scope `parsed_kinds:` and actionable `next_query:`.
    let ModelToolOutcome::Completed(zero_kind) = run(
        "c12",
        "(file-outline \"crates/core/src/engine.rs\" kind: \"enum\")",
    )
    .await
    else {
        panic!("expected Completed for zero-hit kind-filtered outline");
    };
    assert!(
        zero_kind.contains("parsed_kinds:") && zero_kind.contains("next_query: (file-outline \"crates/core/src/engine.rs\""),
        "expected parsed_kinds histogram and actionable next_query on 0-hit kind filter: {zero_kind}"
    );

    // 14. `(workspace-map ...)` produces a compact Turn-1 crate/directory bird's-eye topology summary.
    let ModelToolOutcome::Completed(ws_map) = run(
        "c13",
        "(workspace-map \"\" depth: 2 exclude-tests?: #t limit: 16)",
    )
    .await
    else {
        panic!("expected Completed for workspace-map");
    };
    assert!(
        ws_map.contains("mode: workspace-map")
            && ws_map.contains("crates/core/")
            && ws_map.contains("next_query:"),
        "expected compact workspace-map bird's-eye view: {ws_map}"
    );

    // 15. `(file-outline ... projection: "callable-skeleton")` emits Lstart..Lend spans + folded signatures.
    let ModelToolOutcome::Completed(skeleton) = run(
        "c14",
        "(file-outline \"crates/core/src/engine.rs\" projection: \"callable-skeleton\")",
    )
    .await
    else {
        panic!("expected Completed for callable-skeleton outline");
    };
    assert!(
        skeleton.contains("SearchEngine :: pub struct SearchEngine")
            && skeleton.contains("run_engine :: pub fn run_engine"),
        "expected callable-skeleton folded signatures: {skeleton}"
    );

    // 16. Multi-symbol `(read-file ... symbols: ("SearchEngine" "run_engine"))` fetches both in one RAM call.
    let ModelToolOutcome::Completed(multi_sym) = run(
        "c15",
        "(read-file \"crates/core/src/engine.rs\" symbols: (\"SearchEngine\" \"run_engine\") context: 16)",
    )
    .await
    else {
        panic!("expected Completed for multi-symbol read-file");
    };
    assert!(
        multi_sym.contains("resolved_symbols: 2/2")
            && multi_sym.contains("=== symbol: SearchEngine @")
            && multi_sym.contains("=== symbol: run_engine @"),
        "expected both symbols fetched in one multi-symbol read-file call: {multi_sym}"
    );

    // 17. RAM-accurate `enum` disambiguation on unhydrated Rust files (coarse resident-index `type` -> `enum`).
    let ModelToolOutcome::Completed(unhydrated_enum) = run(
        "c16",
        "(file-outline \"crates/modes/src/\" lang: \"rust\" kind: \"enum\")",
    )
    .await
    else {
        panic!("expected Completed for unhydrated Rust enum outline");
    };
    assert!(
        unhydrated_enum.contains("[enum] EngineMode"),
        "expected RAM-refined enum symbol on unhydrated file: {unhydrated_enum}"
    );

    // 18. `(file-outline ... projection: "kind-summary")` emits compact per-file kind inventory + kind_totals.
    let ModelToolOutcome::Completed(kind_summary) = run(
        "c17",
        "(file-outline \"crates/\" projection: \"kind-summary\" exclude-tests?: #t)",
    )
    .await
    else {
        panic!("expected Completed for kind-summary outline");
    };
    assert!(
        kind_summary.contains("projection: kind-summary")
            && kind_summary.contains("kind_totals:")
            && kind_summary.contains("struct(1): SearchEngine")
            && kind_summary.contains("enum(1): EngineMode"),
        "expected compact kind-summary inventory across hydrated + unhydrated files: {kind_summary}"
    );

    // 19. `(file-outline ... projection: "struct-enum-detail")` emits inline struct fields & enum variants in 1 pass.
    let ModelToolOutcome::Completed(type_detail) = run(
        "c18",
        "(file-outline \"crates/\" projection: \"struct-enum-detail\" kind: (\"struct\" \"enum\") exclude-tests?: #t)",
    )
    .await
    else {
        panic!("expected Completed for struct-enum-detail outline");
    };
    assert!(
        type_detail.contains("SearchEngine :: { pub active: bool }")
            && type_detail.contains("EngineMode :: { Fast, Safe }"),
        "expected inline struct fields and enum variants in struct-enum-detail: {type_detail}"
    );

    // 20. Cursor-aware `(read-file ... cursor: 1 max-lines: 3)` emits exact executable `next_query:` cursor continuation.
    let ModelToolOutcome::Completed(cursor_read) = run(
        "c19",
        "(read-file \"crates/core/src/engine.rs\" cursor: 1 max-lines: 3)",
    )
    .await
    else {
        panic!("expected Completed for cursor-based read-file");
    };
    assert!(
        cursor_read.contains("range: L1..=L3")
            && cursor_read.contains("next_query: (read-file \"crates/core/src/engine.rs\" cursor: 4 max-lines: 120)"),
        "expected exact cursor continuation next_query: {cursor_read}"
    );

    // 21. Truncated `(brute-search ...)` emits `top_files:` and one-hop `(file-outline ...)` suggestion.
    let ModelToolOutcome::Completed(trunc_brute) = run(
        "c20",
        "(brute-search \"pub\" path-prefix: \"crates/\" limit: 1)",
    )
    .await
    else {
        panic!("expected Completed for truncated brute-search");
    };
    assert!(
        trunc_brute.contains("top_files:")
            && trunc_brute.contains("projection: \"callable-skeleton\""),
        "expected top_files and one-hop file-outline transition on truncated brute-search: {trunc_brute}"
    );

    // 22. `struct-enum-detail` defaults to struct/enum kinds even when `kind:` is omitted.
    let ModelToolOutcome::Completed(default_type_detail) = run(
        "c21",
        "(file-outline \"crates/\" projection: \"struct-enum-detail\" exclude-tests?: #t)",
    )
    .await
    else {
        panic!("expected Completed for default-kind struct-enum-detail outline");
    };
    assert!(
        default_type_detail.contains("[struct] SearchEngine")
            && default_type_detail.contains("[enum] EngineMode")
            && !default_type_detail.contains("[function] run_engine"),
        "expected struct-enum-detail without kind: to omit functions: {default_type_detail}"
    );

    // 23. `(workspace-map "./" ...)` normalizes POSIX root alias `./` to workspace root, and
    // `(read-file ... max-lines: 400)` reports `range_clamped: true`.
    let ModelToolOutcome::Completed(root_map) = run("c22", "(workspace-map \"./\" depth: 2)").await
    else {
        panic!("expected Completed for normalized root workspace-map");
    };
    assert!(
        root_map.contains("crates/core/") && !root_map.contains("matched 0 indexed files"),
        "expected ./ to normalize to workspace root: {root_map}"
    );
    let ModelToolOutcome::Completed(clamped_read) = run(
        "c23",
        "(read-file \"crates/core/src/engine.rs\" cursor: 1 max-lines: 400)",
    )
    .await
    else {
        panic!("expected Completed for clamped max-lines read-file");
    };
    assert!(
        clamped_read.contains("range_clamped: true"),
        "expected range_clamped header when max-lines exceeds 200: {clamped_read}"
    );

    // 24. Accidental `(workspace_search "pattern")` wrapper auto-repairs to `(brute-search ...)`.
    let ModelToolOutcome::Completed(repaired_ws) =
        run("c24", "(workspace_search \"SearchEngine\")").await
    else {
        panic!("expected Completed for repaired workspace_search wrapper");
    };
    assert!(
        repaired_ws.contains("crates/core/src/engine.rs") && repaired_ws.contains("repairs:"),
        "expected repaired workspace_search wrapper to match and emit repairs receipt: {repaired_ws}"
    );

    // 25. `(file-outline ... name-contains:/name-all:)` filters symbols in 1 pass, and
    // truncated `(read-file ...)` emits `remaining_symbols:` lookahead jump anchors.
    let ModelToolOutcome::Completed(filtered_outline) = run(
        "c25",
        "(file-outline \"crates/\" name-all: (\"run\" \"engine\") exclude-tests?: #t)",
    )
    .await
    else {
        panic!("expected Completed for name-all filtered file-outline");
    };
    assert!(
        filtered_outline.contains("run_engine") && !filtered_outline.contains("EngineMode"),
        "expected name-all filter on file-outline to narrow to run_engine: {filtered_outline}"
    );
    let ModelToolOutcome::Completed(lookahead_read) = run(
        "c26",
        "(read-file \"crates/core/src/engine.rs\" cursor: 1 max-lines: 2)",
    )
    .await
    else {
        panic!("expected Completed for truncated read-file with symbol lookahead");
    };
    assert!(
        lookahead_read.contains("remaining_symbols:") && lookahead_read.contains("run_engine"),
        "expected remaining_symbols lookahead anchor on truncated read-file: {lookahead_read}"
    );

    // 26. Broad `(brute-search ...)` ranks production files ahead of tests when exclude-tests? is #f,
    // and zero-hit stem fallback surfaces nearest production symbols from the RAM index.
    let ModelToolOutcome::Completed(prod_first_brute) =
        run("c27", "(brute-search \"SearchEngine\" limit: 20)").await
    else {
        panic!("expected Completed for prod-first brute-search");
    };
    let prod_idx = prod_first_brute
        .find("crates/core/src/engine.rs")
        .expect("prod engine.rs hit");
    if let Some(test_idx) = prod_first_brute.find("tests/") {
        assert!(
            prod_idx < test_idx,
            "expected production source hit before tests/ hit: {prod_first_brute}"
        );
    }
    let ModelToolOutcome::Completed(zero_hit_nearest) = run(
        "c28",
        "(brute-search \"EngineMode.*MissingToken|NoSuchEngineMode\" limit: 10)",
    )
    .await
    else {
        panic!("expected Completed for zero-hit nearest symbol diagnostic");
    };
    assert!(
        zero_hit_nearest.contains("live stem_hits:")
            && zero_hit_nearest.contains("nearest_symbols:")
            && zero_hit_nearest.contains("EngineMode"),
        "expected L0 zero-hit nearest_symbols recovery: {zero_hit_nearest}"
    );

    // 27. Truncated multi-file `(file-outline ...)` emits `omitted_files:` inventory, and
    // `(read-file ... symbol: ...)` resolves the complete AST definition body span.
    let ModelToolOutcome::Completed(omitted_outline) = run(
        "c29",
        "(file-outline \"crates/\" exclude-tests?: #t limit: 2 max-per-file: 1)",
    )
    .await
    else {
        panic!("expected Completed for truncated multi-file outline");
    };
    assert!(
        omitted_outline.contains("truncated: true")
            && omitted_outline.contains("omitted_files:"),
        "expected omitted_files inventory on truncated directory outline: {omitted_outline}"
    );
    let ModelToolOutcome::Completed(ast_span_read) = run(
        "c30",
        "(read-file \"crates/modes/src/lib.rs\" symbol: \"EngineMode\" context: 2)",
    )
    .await
    else {
        panic!("expected Completed for AST-span symbol read-file");
    };
    assert!(
        ast_span_read.contains("symbol_span: L1..=L4 (complete AST body)")
            && ast_span_read.contains("enum EngineMode")
            && ast_span_read.contains("Safe,"),
        "expected full AST symbol_span body resolution: {ast_span_read}"
    );
    let ModelToolOutcome::Completed(excluded_sample_brute) = run(
        "c31",
        "(brute-search \"SearchEngine\" exclude-tests?: #t limit: 10)",
    )
    .await
    else {
        panic!("expected Completed for exclude-tests sample brute-search");
    };
    assert!(
        excluded_sample_brute.contains("excluded_files: 1")
            && excluded_sample_brute.contains("hidden_test_dirs: crates/core(1)"),
        "expected excluded_files header to expose hidden test crate counts: {excluded_sample_brute}"
    );

    // 28. `(workspace-map "crates/" depth: 1)` auto-expands depth past the prefix segment
    // so callers immediately see child crates (`crates/core/`, `crates/modes/`) instead of a single bucket.
    let ModelToolOutcome::Completed(shallow_map) =
        run("c32", "(workspace-map \"crates/\" depth: 1)").await
    else {
        panic!("expected Completed for auto-expanded workspace-map depth");
    };
    assert!(
        shallow_map.contains("crates/core/") && shallow_map.contains("crates/modes/"),
        "expected workspace-map on crates/ with depth: 1 to auto-expand to child crates: {shallow_map}"
    );

    // 29. `(read-file "crates/<crate>/src/...")` emits `crate_tests:` companion test files
    // and multi-symbol headers embed full repo-relative `path:Lline` anchors.
    let ModelToolOutcome::Completed(companion_read) = run(
        "c33",
        "(read-file \"crates/core/src/engine.rs\" symbols: (\"SearchEngine\" \"run_engine\"))",
    )
    .await
    else {
        panic!("expected Completed for multi-symbol read-file with crate_tests");
    };
    assert!(
        companion_read.contains("crate_tests: crates/core/tests/engine_test.rs")
            && companion_read.contains("=== symbol: SearchEngine @ crates/core/src/engine.rs:L1"),
        "expected crate_tests header and full path:line symbol anchors: {companion_read}"
    );
}
}
