// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit regressions for L0 pure-Rust RAM primitives: `exclude-path-prefix:`,
//! zero-hit diagnostics, `(file-outline ...)`, and `(read-file ...)`.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
use marlin_agent_runtime::{
    ModelToolDeclaration, ModelToolOutcome, SearchToolDeclarationRequest, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, ToolRuntime,
};
use marlin_agent_search_engine::{
    WorkspaceEdbEntityFact, WorkspaceIndexConfig, WorkspaceOpenEdbIngestBatch,
    WorkspaceSearchIndex,
};
use marlin_agent_tooling::{ResidentSearchToolHandler, SharedSearchIndexSlot};

#[tokio::test(flavor = "current_thread")]
async fn l0_exclude_path_prefix_outline_read_file_and_zero_hit_diagnostics() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let files: [(&str, &str); 4] = [
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

    let slot = SharedSearchIndexSlot::with_sync_debounce(
        Some(Arc::new(RwLock::new(index))),
        Duration::from_secs(60),
    );
    let declaration = ModelToolDeclaration::search(SearchToolDeclarationRequest {
        name: ModelToolName::new("workspace_search").expect("name"),
        description: "search".to_owned(),
        argument: "query".to_owned(),
        timeout_seconds: 10,
        output_limit: 16_384,
    })
    .expect("decl");
    let handler = ResidentSearchToolHandler::new(tmp.path().to_path_buf(), declaration, slot);
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(4));

    let run = |id: &str, query: &str| {
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
}
