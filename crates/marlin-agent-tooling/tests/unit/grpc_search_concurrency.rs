// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Concurrent HTTP/2 `tonic` gRPC search tests (16-agent mixed L0+L1+L2 + Gap regressions).

#[cfg(feature = "grpc")]
mod grpc_concurrency_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, RwLock};
    use std::time::{Duration, Instant};

    use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
    use marlin_agent_runtime::{
        ModelToolDeclaration, ModelToolOutcome, SearchToolDeclarationRequest, TokioAgentRuntime,
        TokioAgentRuntimeBuildRequest, ToolRuntime,
    };
    use marlin_agent_search_engine::{
        SearchDslSessionId, SearchPlanProjectionError, WorkspaceEdbAttr, WorkspaceEdbEdgeFact,
        WorkspaceEdbEntityFact, WorkspaceIndexConfig, WorkspaceOpenEdbIngestBatch,
        WorkspaceSearchIndex,
    };
    use marlin_agent_tooling::{
        AsyncSearchDslSessionRunner, GrpcSearchToolHandler, LoopbackSearchGrpcServer,
        SearchGatewayIndexHandle, SearchGatewayService, SearchQueryTier,
        tool_schema_entries_from_declarations,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn sixteen_concurrent_agents_mixed_l0_l1_l2_grpc_zero_deadlock() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::create_dir_all(tmp.path().join("tests")).unwrap();
        std::fs::write(
            tmp.path().join("src/app.rs"),
            "pub fn local_init() -> usize { 1 }\n\
             pub fn dispatch_search_batch() -> usize {\n\
                 let x = local_init();\n\
                 let y = parse_token();\n\
                 x + y\n\
             }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("src/parser_a.rs"),
            "pub fn parse_token() -> usize { 10 }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("src/parser_b.rs"),
            "pub fn parse_token() -> usize { 20 }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("tests/mock_parser.rs"),
            "pub fn parse_token() -> usize { 99 }\n",
        )
        .unwrap();

        let mut idx =
            WorkspaceSearchIndex::build_with_config(tmp.path(), WorkspaceIndexConfig::default());
        idx.ingest_open_edb_facts(WorkspaceOpenEdbIngestBatch {
            language_id: Some("rust".to_owned()),
            provider_id: Some("asp-rust".to_owned()),
            entities: vec![
                WorkspaceEdbEntityFact {
                    entity_id: "src/app.rs#fn:local_init".to_owned(),
                    file_id: 0,
                    path: "src/app.rs".to_owned(),
                    line: 1,
                    name: "local_init".to_owned(),
                    language: "rust".to_owned(),
                    kind: "function".to_owned(),
                    language_kind: "fn_item".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: Vec::new(),
                },
                WorkspaceEdbEntityFact {
                    entity_id: "src/app.rs#fn:dispatch_search_batch".to_owned(),
                    file_id: 0,
                    path: "src/app.rs".to_owned(),
                    line: 2,
                    name: "dispatch_search_batch".to_owned(),
                    language: "rust".to_owned(),
                    kind: "function".to_owned(),
                    language_kind: "fn_item".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: Vec::new(),
                },
                WorkspaceEdbEntityFact {
                    entity_id: "src/parser_a.rs#fn:parse_token".to_owned(),
                    file_id: 0,
                    path: "src/parser_a.rs".to_owned(),
                    line: 1,
                    name: "parse_token".to_owned(),
                    language: "rust".to_owned(),
                    kind: "function".to_owned(),
                    language_kind: "fn_item".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: Vec::new(),
                },
                WorkspaceEdbEntityFact {
                    entity_id: "src/parser_b.rs#fn:parse_token".to_owned(),
                    file_id: 0,
                    path: "src/parser_b.rs".to_owned(),
                    line: 1,
                    name: "parse_token".to_owned(),
                    language: "rust".to_owned(),
                    kind: "function".to_owned(),
                    language_kind: "fn_item".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: Vec::new(),
                },
                WorkspaceEdbEntityFact {
                    entity_id: "tests/mock_parser.rs#fn:parse_token".to_owned(),
                    file_id: 0,
                    path: "tests/mock_parser.rs".to_owned(),
                    line: 1,
                    name: "parse_token".to_owned(),
                    language: "rust".to_owned(),
                    kind: "function".to_owned(),
                    language_kind: "fn_item".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: Vec::new(),
                },
            ],
            edges: vec![
                WorkspaceEdbEdgeFact {
                    src: "src/app.rs#fn:dispatch_search_batch".to_owned(),
                    dst: "local_init".to_owned(),
                    relation: "calls".to_owned(),
                    path: "src/app.rs".to_owned(),
                    line: 3,
                },
                WorkspaceEdbEdgeFact {
                    src: "src/app.rs#fn:dispatch_search_batch".to_owned(),
                    dst: "parse_token".to_owned(),
                    relation: "calls".to_owned(),
                    path: "src/app.rs".to_owned(),
                    line: 4,
                },
            ],
            ..Default::default()
        });

        let warm = Arc::new(RwLock::new(idx));
        let index_handle = SearchGatewayIndexHandle::with_sync_debounce(
            Some(tmp.path().to_path_buf()),
            Some(warm),
            Duration::from_secs(5),
        );

        let vm_calls = Arc::new(AtomicUsize::new(0));
        let l1_calls = Arc::new(AtomicUsize::new(0));
        let l2_calls = Arc::new(AtomicUsize::new(0));
        let vm_calls_ref = Arc::clone(&vm_calls);
        let l1_ref = Arc::clone(&l1_calls);
        let l2_ref = Arc::clone(&l2_calls);
        let sync_runner = crate::projection::shared_test_search_dsl_session_runner();
        let async_runner: AsyncSearchDslSessionRunner =
            Arc::new(move |shared_index, query, limit, tier, sid| {
                vm_calls_ref.fetch_add(1, Ordering::Relaxed);
                match tier {
                    SearchQueryTier::FastAscent => {
                        l1_ref.fetch_add(1, Ordering::Relaxed);
                    }
                    SearchQueryTier::DusaSolver => {
                        l2_ref.fetch_add(1, Ordering::Relaxed);
                    }
                    SearchQueryTier::BruteForce => unreachable!("L0 must never reach Gerbil VM"),
                }
                let runner = Arc::clone(&sync_runner);
                Box::pin(async move {
                    tokio::task::spawn_blocking(move || runner(shared_index, &query, limit, &sid))
                        .await
                        .map_err(|_| SearchPlanProjectionError::Invalid("join error"))?
                })
            });

        let decl = ModelToolDeclaration::search(SearchToolDeclarationRequest {
            name: ModelToolName::new("workspace_search").unwrap(),
            description: "Resident search".into(),
            argument: "query".into(),
            timeout_seconds: 15,
            output_limit: 8192,
        })
        .unwrap();
        let (owner, _events) =
            TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(32));
        let service = SearchGatewayService::with_lazy_async_runner(
            index_handle.clone(),
            tool_schema_entries_from_declarations(std::slice::from_ref(&decl)),
            async_runner,
            owner.context(),
        );
        let server = LoopbackSearchGrpcServer::spawn(service, &owner).expect("spawn loopback gRPC");
        let handler = Arc::new(GrpcSearchToolHandler::new(
            decl,
            server.client(),
            Arc::new(RwLock::new(SearchDslSessionId::new("proj-16").unwrap())),
        ));

        let l0_query = "(brute-search \"dispatch_search_batch\" path-prefix: \"src/\" lang: \"rust\")";
        let l1_query = "(ascent-fragment ((answer path name line) <-- (file f \"rust\" path) (rust_fn _ f name line)))";
        let l2_query = "(search-choice-program (use rust) (choice resolve (callsite target)) (rule (closed (resolve ?call) (?def)) (ref-at ?call ?name) (same-module-def ?call ?name ?def)) (rule (open (resolve ?call) ?def) (ref-at ?call ?name) (workspace-def ?name ?def)) (rule (closed (resolve ?call) ()) (resolve ?call ?def) (in-test-file ?def)) (solve resolve state-budget: 32 provenance?: #t))";

        let started = Instant::now();
        let mut handles = Vec::with_capacity(16);
        for agent_idx in 0..16usize {
            let handler = Arc::clone(&handler);
            let ctx = owner.context();
            let query = match agent_idx % 3 {
                0 => l0_query,
                1 => l1_query,
                _ => l2_query,
            };
            handles.push(tokio::spawn(async move {
                let t0 = Instant::now();
                let call = ModelToolCall {
                    id: ModelToolCallId::new(format!("agent-{agent_idx}")).unwrap(),
                    name: ModelToolName::new("workspace_search").unwrap(),
                    arguments: serde_json::Map::from_iter([(
                        "query".to_owned(),
                        serde_json::Value::String(query.to_owned()),
                    )]),
                };
                let outcome = handler.run_tool(call, ctx).await;
                (agent_idx % 3, t0.elapsed().as_micros(), outcome)
            }));
        }

        let mut l0_latencies = Vec::new();
        let mut l1_latencies = Vec::new();
        let mut l2_latencies = Vec::new();
        for handle in handles {
            let (bucket, us, outcome) = handle.await.expect("agent task joined");
            match (bucket, outcome) {
                (0, ModelToolOutcome::Completed(body)) => {
                    assert!(body.contains("mode: brute-force"), "{body}");
                    assert!(body.contains("src/app.rs"), "{body}");
                    l0_latencies.push(us);
                }
                (1, ModelToolOutcome::Completed(body)) => {
                    assert!(body.contains("mode: ascent"), "{body}");
                    assert!(body.contains("dispatch_search_batch"), "{body}");
                    l1_latencies.push(us);
                }
                (2, ModelToolOutcome::Completed(body)) => {
                    assert!(body.contains("mode: choice"), "{body}");
                    assert!(body.contains("worlds: 2 (SAT"), "{body}");
                    assert!(body.contains("[deduction: closed"), "{body}");
                    assert!(body.contains("[branch: open"), "{body}");
                    assert!(!body.contains("tests/mock_parser.rs"), "{body}");
                    l2_latencies.push(us);
                }
                other => panic!("unexpected outcome for bucket: {other:?}"),
            }
        }

        let total_us = started.elapsed().as_micros();
        let metrics = index_handle.scheduler_metrics();
        assert_eq!(metrics.l0_brute_queries, 6);
        assert_eq!(metrics.l1_ascent_queries, 5);
        assert_eq!(metrics.l2_dusa_queries, 5);
        assert_eq!(
            vm_calls.load(Ordering::Relaxed),
            10,
            "only L1 (5) + L2 (5) hit the Gerbil VM; L0 (6) never touches VM! metrics={metrics:?}"
        );
        assert_eq!(
            metrics.disk_sync_executed, 1,
            "16 agents must coalesce into 1 disk sync"
        );
        assert_eq!(metrics.disk_sync_coalesced_or_debounced, 15);

        println!(
            "\n=== 16-AGENT CONCURRENT MIXED HTTP/2 GRPC SEARCH BENCHMARK (L0+L1+L2) ===\n\
             Total wall time (16 agents)  : {total_us}us\n\
             L0 Brute-Force (6 agents)    : latencies_us={l0_latencies:?} (0 Gerbil VM calls)\n\
             L1 Case A Ascent (5 agents)  : latencies_us={l1_latencies:?} ({} VM calls)\n\
             L2 Case B Dusa (5 agents)    : latencies_us={l2_latencies:?} ({} VM calls)\n\
             Disk syncs executed          : {} / 16 (coalesced/debounced={})\n\
             =============================================================",
            l1_calls.load(Ordering::Relaxed),
            l2_calls.load(Ordering::Relaxed),
            metrics.disk_sync_executed,
            metrics.disk_sync_coalesced_or_debounced,
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn search_gaps_regression_file_join_impl_trait_multiline_regex_coverage_and_validation() {
        let tmp = tempfile::tempdir().expect("temp workspace");
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(
            tmp.path().join("src/archive.rs"),
            "// SPDX header\npub struct AotArchiveLinkReceipt;\npub trait ModelGateway {}\nimpl ModelGateway for AotArchiveLinkReceipt {}\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("src/other.rs"),
            "// SPDX header\npub struct ExtraStruct;\npub fn alpha() {}\npub fn beta() {}\n",
        )
        .unwrap();

        let mut idx =
            WorkspaceSearchIndex::build_with_config(tmp.path(), WorkspaceIndexConfig::default());
        idx.ingest_open_edb_facts(WorkspaceOpenEdbIngestBatch {
            schema_id: None,
            schema_version: None,
            language_id: Some("rust".into()),
            provider_id: Some("asp-rust".into()),
            entities: vec![
                WorkspaceEdbEntityFact {
                    entity_id: "src/archive.rs#struct:AotArchiveLinkReceipt".into(),
                    file_id: 0,
                    path: "src/archive.rs".into(),
                    line: 2,
                    name: "AotArchiveLinkReceipt".into(),
                    language: "rust".into(),
                    kind: "struct".into(),
                    language_kind: "struct_item".into(),
                    source: "native-parser".into(),
                    attrs: Vec::new(),
                },
                WorkspaceEdbEntityFact {
                    entity_id: "src/archive.rs#trait:ModelGateway".into(),
                    file_id: 0,
                    path: "src/archive.rs".into(),
                    line: 3,
                    name: "ModelGateway".into(),
                    language: "rust".into(),
                    kind: "trait".into(),
                    language_kind: "trait_item".into(),
                    source: "native-parser".into(),
                    attrs: Vec::new(),
                },
                WorkspaceEdbEntityFact {
                    entity_id: "src/archive.rs#impl:AotArchiveLinkReceipt".into(),
                    file_id: 0,
                    path: "src/archive.rs".into(),
                    line: 4,
                    name: "AotArchiveLinkReceipt".into(),
                    language: "rust".into(),
                    kind: "item".into(),
                    language_kind: "impl_item".into(),
                    source: "native-parser".into(),
                    attrs: vec![WorkspaceEdbAttr {
                        key: "implements".into(),
                        value: "ModelGateway".into(),
                    }],
                },
                WorkspaceEdbEntityFact {
                    entity_id: "src/other.rs#struct:ExtraStruct".into(),
                    file_id: 0,
                    path: "src/other.rs".into(),
                    line: 2,
                    name: "ExtraStruct".into(),
                    language: "rust".into(),
                    kind: "struct".into(),
                    language_kind: "struct_item".into(),
                    source: "native-parser".into(),
                    attrs: Vec::new(),
                },
            ],
            edges: vec![WorkspaceEdbEdgeFact {
                src: "src/archive.rs#impl:AotArchiveLinkReceipt".into(),
                relation: "implements".into(),
                dst: "ModelGateway".into(),
                path: "src/archive.rs".into(),
                line: 4,
            }],
        });

        let decl = ModelToolDeclaration::search(SearchToolDeclarationRequest {
            name: ModelToolName::new("workspace_search").unwrap(),
            description: "Resident search".into(),
            argument: "query".into(),
            timeout_seconds: 15,
            output_limit: 8192,
        })
        .unwrap();
        let (owner, _events) =
            TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));
        let service = SearchGatewayService::new(
            Arc::new(RwLock::new(idx)),
            tool_schema_entries_from_declarations(std::slice::from_ref(&decl)),
            crate::projection::shared_test_search_dsl_session_runner(),
            owner.context(),
        );
        let server = LoopbackSearchGrpcServer::spawn(service, &owner).expect("spawn loopback gRPC");
        let handler = GrpcSearchToolHandler::new(
            decl,
            server.client(),
            Arc::new(RwLock::new(SearchDslSessionId::new("proj-gaps").unwrap())),
        );

        let run = |id: &str, query: &str| {
            let call = ModelToolCall {
                id: ModelToolCallId::new(id).unwrap(),
                name: ModelToolName::new("workspace_search").unwrap(),
                arguments: serde_json::Map::from_iter([(
                    "query".to_owned(),
                    serde_json::Value::String(query.to_owned()),
                )]),
            };
            handler.run_tool(call, owner.context())
        };

        // Gap 2a: exact `file` path literal join must NOT pollute trigram tokens with path literal!
        let file_join = match run(
            "gap-2a",
            "(ascent-fragment ((answer name line) <-- (file f \"rust\" \"src/archive.rs\") (rust_struct _ f name line)))",
        )
        .await
        {
            ModelToolOutcome::Completed(out) => out,
            ModelToolOutcome::Failed(err) => {
                panic!("expected Completed for exact file path join, got Failed: {err}")
            }
        };
        assert!(file_join.contains("AotArchiveLinkReceipt"), "{file_join}");
        assert!(file_join.contains("scope_coverage:"), "{file_join}");
        assert!(file_join.contains(">L"), "{file_join}");
        assert!(file_join.contains("returned: 1"), "{file_join}");
        assert!(file_join.contains("truncated: false"), "{file_join}");

        // Gap 2b: reverse join `(rust_struct _ f "AotArchiveLinkReceipt" line) (file f _ path)`!
        let reverse_join = match run(
            "gap-2b",
            "(ascent-fragment ((answer path line) <-- (rust_struct _ f \"AotArchiveLinkReceipt\" line) (file f _ path)))",
        )
        .await
        {
            ModelToolOutcome::Completed(out) => out,
            ModelToolOutcome::Failed(err) => {
                panic!("expected Completed for reverse file join, got Failed: {err}")
            }
        };
        assert!(reverse_join.contains("src/archive.rs"), "{reverse_join}");

        // Gap 4: `impl_trait` answers "who implements ModelGateway?" directly!
        let impl_trait_out = match run(
            "gap-4",
            "(ascent-fragment ((answer ty path line) <-- (impl_trait ty \"ModelGateway\" path line)))",
        )
        .await
        {
            ModelToolOutcome::Completed(out) => out,
            ModelToolOutcome::Failed(err) => {
                panic!("expected Completed for impl_trait, got Failed: {err}")
            }
        };
        assert!(
            impl_trait_out.contains("AotArchiveLinkReceipt"),
            "{impl_trait_out}"
        );
        assert!(impl_trait_out.contains("src/archive.rs"), "{impl_trait_out}");

        // Gap 5 & Gap 7a: `((limit 1))` auto-repairs cleanly and reports total vs returned + truncated!
        let ModelToolOutcome::Completed(limited_out) = run(
            "gap-5-7a",
            "(ascent-fragment ((limit 1)) ((answer name line) <-- (rust_struct _ _ name line)))",
        )
        .await
        else {
            panic!("expected Completed for ((limit 1))");
        };
        assert!(limited_out.contains("results: 2"), "{limited_out}");
        assert!(limited_out.contains("returned: 1"), "{limited_out}");
        assert!(limited_out.contains("truncated: true"), "{limited_out}");
        assert!(
            limited_out.contains("unwrap-limit-clause"),
            "{limited_out}"
        );

        // Gap 7b: illegal clause returns crisp Plan Validation error (NOT opaque native status 3!)
        let ModelToolOutcome::Failed(err) = run(
            "gap-7b",
            "(ascent-fragment (unknown-directive 99) ((answer p) <-- (file _ _ p)))",
        )
        .await
        else {
            panic!("expected Failed for illegal ascent clause");
        };
        assert!(err.contains("search plan validation failed"), "{err}");
        assert!(err.contains("DSL_REPAIR_DIAGNOSTIC"), "{err}");
        assert!(err.contains("invalid_ascent_clause"), "{err}");
        assert!(!err.contains("native status 3"), "{err}");

        // Gap 6a: `re:^impl` matches line-anchored `impl ModelGateway ...` via native multiline mode!
        let ModelToolOutcome::Completed(re_anchor) =
            run("gap-6a", "(brute-search \"re:^impl \" lang: \"rust\")").await
        else {
            panic!("expected Completed for re:^impl");
        };
        assert!(re_anchor.contains("src/archive.rs:4"), "{re_anchor}");
        assert!(
            !re_anchor.contains("enable-multiline-anchors"),
            "{re_anchor}"
        );

        // Gap 6b: unprefixed `"^impl "` emits actionable diagnostic hint instead of silent 0!
        let ModelToolOutcome::Completed(unprefixed) =
            run("gap-6b", "(brute-search \"^impl \" lang: \"rust\")").await
        else {
            panic!("expected Completed for unprefixed ^impl");
        };
        assert!(unprefixed.contains("hits: 0"), "{unprefixed}");
        assert!(unprefixed.contains("diagnostic:"), "{unprefixed}");
        assert!(unprefixed.contains("re:^impl"), "{unprefixed}");

        // Live-eval regression: regex escapes (\s, \w), multi-line matches per file,
        // bare brute-search auto-wrap, and `scope:` keyword alias!
        let ModelToolOutcome::Completed(re_escapes) = run(
            "live-re-escapes",
            "brute-search \"re:pub\\s+(struct|trait)\\s+\\w+\" scope: \"src/archive.rs\" context: 0",
        )
        .await
        else {
            panic!("expected Completed for bare brute-search with regex escapes & scope:");
        };
        assert!(re_escapes.contains("wrap-brute-parens"), "{re_escapes}");
        assert!(re_escapes.contains("canonical_query:"), "{re_escapes}");
        assert!(re_escapes.contains("2 match(es)"), "{re_escapes}");
        assert!(
            re_escapes.contains("AotArchiveLinkReceipt"),
            "{re_escapes}"
        );
        assert!(
            re_escapes.contains("pub trait ModelGateway"),
            "{re_escapes}"
        );
    }
}
