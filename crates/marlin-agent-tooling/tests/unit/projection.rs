// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCapability, ModelToolDeclaration, ModelToolGovernance, ModelToolOutcome,
    SearchToolDeclarationRequest, ShellToolDeclarationRequest, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, ToolRuntime,
};
use marlin_agent_search_engine::{
    SearchDslExecutionRequest, SearchDslRunner, SearchPlanProjectionError, WorkspaceEdbAttr,
    WorkspaceEdbEntityFact, WorkspaceIndexConfig, WorkspaceOpenEdbIngestBatch,
    WorkspaceSearchIndex, default_marlin_scheme_dir, default_workspace_snapshot_path,
    execute_package_search_dsl,
};
use marlin_agent_tooling::{
    ResidentSearchToolHandler, SharedSearchIndexSlot, catalog_from_declarations_with_search,
    decode_tooling,
};
use std::sync::{Arc, OnceLock, RwLock, mpsc};

type TestVmJob = Box<dyn FnOnce(&GerbilRuntime) + Send>;

fn with_shared_test_gerbil_vm<R: Send + 'static>(
    f: impl FnOnce(&GerbilRuntime) -> R + Send + 'static,
) -> R {
    static WORKER: OnceLock<mpsc::Sender<TestVmJob>> = OnceLock::new();
    let tx = WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<TestVmJob>();
        std::thread::Builder::new()
            .name("tooling-test-gerbil-vm".to_owned())
            .spawn(move || {
                let runtime = GerbilRuntime::initialize().expect("initialize test Gerbil VM");
                while let Ok(job) = rx.recv() {
                    job(&runtime);
                }
            })
            .expect("spawn test Gerbil VM thread");
        tx
    });
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    tx.send(Box::new(move |rt| {
        let _ = reply_tx.send(f(rt));
    }))
    .expect("send test VM job");
    reply_rx.recv().expect("receive test VM result")
}

fn shared_test_search_dsl_runner() -> SearchDslRunner {
    Arc::new(|shared_index, query, output_limit| {
        let scheme_dir = default_marlin_scheme_dir()
            .ok_or(SearchPlanProjectionError::Invalid("missing scheme dir"))?
            .to_string_lossy()
            .into_owned();
        let query = query.to_owned();
        with_shared_test_gerbil_vm(move |rt| {
            let guard = shared_index
                .read()
                .map_err(|_| SearchPlanProjectionError::Invalid("index lock poisoned"))?;
            let req = SearchDslExecutionRequest {
                index: &guard,
                query: &query,
                output_limit,
            };
            execute_package_search_dsl(rt, &scheme_dir, &req)
        })
    })
}

fn vector<'a>(
    runtime: &'a GerbilRuntime,
    objects: &[RootedSchemeObject<'a>],
) -> RootedSchemeObject<'a> {
    runtime
        .collection_from_objects(CollectionKind::Vector, &objects.iter().collect::<Vec<_>>())
        .unwrap()
        .into_object()
}

fn symbol<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime
        .intern_identifier(IdentifierKind::Symbol, value)
        .unwrap()
}

fn declaration<'a>(
    runtime: &'a GerbilRuntime,
    name: &str,
    governance: &str,
) -> RootedSchemeObject<'a> {
    vector(
        runtime,
        &[
            symbol(runtime, name),
            runtime
                .string_from_utf8("bounded command")
                .unwrap()
                .into_object(),
            symbol(runtime, "shell"),
            symbol(runtime, governance),
            symbol(runtime, "command"),
            runtime.scalar_object(SchemeScalar::Fixnum(5)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(1024)).unwrap(),
        ],
    )
}

#[test]
fn native_projection_admits_only_unique_ask_declarations() {
    with_shared_test_gerbil_vm(|runtime| {
        assert_eq!(
            decode_tooling(vector(runtime, &[declaration(runtime, "shell-one", "ask")]))
                .unwrap()
                .len(),
            1
        );
        assert!(
            decode_tooling(vector(
                runtime,
                &[declaration(runtime, "shell-one", "allow")]
            ))
            .is_err()
        );
        assert!(
            decode_tooling(vector(
                runtime,
                &[
                    declaration(runtime, "shell-one", "ask"),
                    declaration(runtime, "shell-one", "ask"),
                ],
            ))
            .is_err()
        );

        let search_decl = vector(
            runtime,
            &[
                symbol(runtime, "workspace_search"),
                runtime
                    .string_from_utf8("resident search")
                    .unwrap()
                    .into_object(),
                symbol(runtime, "search"),
                symbol(runtime, "ask"),
                symbol(runtime, "query"),
                runtime.scalar_object(SchemeScalar::Fixnum(10)).unwrap(),
                runtime.scalar_object(SchemeScalar::Fixnum(4096)).unwrap(),
            ],
        );
        let decoded = decode_tooling(vector(runtime, &[search_decl])).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].capability(), ModelToolCapability::Search);
        let catalog = catalog_from_declarations_with_search(
            decoded,
            std::path::PathBuf::from("."),
            SharedSearchIndexSlot::empty(),
        )
        .unwrap();
        assert_eq!(catalog.offer().definitions().len(), 1);
    });
}

#[tokio::test]
async fn resident_search_tool_handler_executes_symbol_and_graph_queries_with_generation_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(
        tmp.path().join("src/kernel.rs"),
        "pub fn resident_kernel_entry() -> usize { 7 }\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("src/caller.rs"),
        "use crate::kernel::resident_kernel_entry;\npub fn invoke() { let _ = resident_kernel_entry(); }\n",
    )
    .unwrap();

    let warm = Arc::new(RwLock::new(WorkspaceSearchIndex::build_with_config(
        tmp.path(),
        WorkspaceIndexConfig::default(),
    )));
    let slot = SharedSearchIndexSlot::from_index(warm.clone());
    slot.attach_dsl_runner(shared_test_search_dsl_runner());
    let decl = marlin_agent_runtime::ModelToolDeclaration::search(SearchToolDeclarationRequest {
        name: ModelToolName::new("workspace_search").unwrap(),
        description: "Resident search tool".into(),
        argument: "query".into(),
        timeout_seconds: 10,
        output_limit: 8192,
    })
    .unwrap();
    let handler = ResidentSearchToolHandler::new(tmp.path().to_path_buf(), decl, slot);
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));

    let sym_call = ModelToolCall {
        id: ModelToolCallId::new("call-sym-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "(ascent-fragment (relation answer ((path string?) (line integer?))) ((answer path line) <-- (file f \"rust\" path) (rust_fn id f \"resident_kernel_entry\" line)))"
                    .to_owned(),
            ),
        )]),
    };
    let outcome = handler.run_tool(sym_call, owner.context()).await;
    match outcome {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("generation: blake3-256:"), "{body}");
            assert!(body.contains("mode: ascent"), "{body}");
            assert!(body.contains("src/kernel.rs"), "{body}");
        }
        other => panic!("expected Completed, got {other:?}"),
    }

    // Legacy @lang:/@sym: string syntax must be rejected fail-closed (zero compatibility shim).
    let legacy_call = ModelToolCall {
        id: ModelToolCallId::new("call-legacy-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String("@lang:rust @sym:resident_kernel_entry".to_owned()),
        )]),
    };
    match handler.run_tool(legacy_call, owner.context()).await {
        ModelToolOutcome::Failed(err) => {
            assert!(err.contains("ascent-fragment") && err.contains("retired"), "{err}");
        }
        other => panic!("expected Failed for retired @sym: syntax, got {other:?}"),
    }

    // Real upstream Dusa POPL 2025 `relational-choice-program` joining POO-derived `rust_fn` zero-eval!
    let choice_call = ModelToolCall {
        id: ModelToolCallId::new("call-choice-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "(relational-choice-program (choice role (f r)) (rule (closed (role ?f) ((value 'kernel) (value 'caller))) (file ?f (value 'rust) ?_)) (rule (closed (role ?f) ((value 'kernel))) (rust_fn ?_ ?f (value 'resident_kernel_entry) ?_)) (limits 4096 4096 4096))"
                    .to_owned(),
            ),
        )]),
    };
    match handler.run_tool(choice_call, owner.context()).await {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("mode: choice"), "{body}");
            assert!(body.contains("kernel"), "{body}");
        }
        other => panic!("expected Completed for choice query, got {other:?}"),
    }

    // Safe Pre-Tool-Call Model AST Repair Hook: heals markdown fence, missing `)`,
    // `<-` arrow, and untyped columns while reporting an auditable `repairs:` receipt!
    let repaired_call = ModelToolCall {
        id: ModelToolCallId::new("call-repair-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "```scheme\n(ascent-fragment (relation answer (path line)) ((answer path line) <- (file f \"rust\" path) (rust_fn id f \"resident_kernel_entry\" line))\n```"
                    .to_owned(),
            ),
        )]),
    };
    match handler.run_tool(repaired_call, owner.context()).await {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("src/kernel.rs"), "{body}");
            assert!(body.contains("repairs:"), "{body}");
            assert!(body.contains("strip-code-fence"), "{body}");
            assert!(body.contains("close-missing-parens"), "{body}");
            assert!(body.contains("normalize-rule-arrow"), "{body}");
            assert!(body.contains("infer-column-predicates"), "{body}");
        }
        other => panic!("expected Completed after safe AST repair hook, got {other:?}"),
    }

    // Native-parser Open EDB ingestion (`semantic-open-edb.v1`): distinguishes `impl_item`
    // and `macro_rules` cleanly via POO `(rust_impl ...)` and `(rust_macro ...)` zero-regex!
    {
        let mut guard = warm.write().unwrap();
        let receipt = guard.ingest_open_edb_facts(WorkspaceOpenEdbIngestBatch {
            language_id: Some("rust".to_owned()),
            provider_id: Some("asp-rust".to_owned()),
            entities: vec![
                WorkspaceEdbEntityFact {
                    entity_id: String::new(),
                    file_id: 0,
                    path: "src/kernel.rs".to_owned(),
                    line: 1,
                    name: "KernelExecutor".to_owned(),
                    language: "rust".to_owned(),
                    kind: "item".to_owned(),
                    language_kind: "impl_item".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: vec![WorkspaceEdbAttr {
                        key: "trait".to_owned(),
                        value: "ToolRuntime".to_owned(),
                    }],
                },
                WorkspaceEdbEntityFact {
                    entity_id: String::new(),
                    file_id: 0,
                    path: "src/kernel.rs".to_owned(),
                    line: 2,
                    name: "define_kernel_route".to_owned(),
                    language: "rust".to_owned(),
                    kind: "macro".to_owned(),
                    language_kind: "macro_rules".to_owned(),
                    source: "native-parser".to_owned(),
                    attrs: Vec::new(),
                },
            ],
            ..Default::default()
        });
        assert_eq!(receipt.updated_files, 1);
        assert_eq!(receipt.ingested_entities, 2);
    }

    let native_impl_call = ModelToolCall {
        id: ModelToolCallId::new("call-impl-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "((answer name trait_name) <-- (rust_impl id _ name _) (attr id \"trait\" trait_name))"
                    .to_owned(),
            ),
        )]),
    };
    match handler.run_tool(native_impl_call, owner.context()).await {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("KernelExecutor"), "{body}");
            assert!(body.contains("ToolRuntime"), "{body}");
            assert!(body.contains("wrap-bare-rule"), "{body}");
            assert!(body.contains("canonical_query:"), "{body}");
            assert!(!body.contains("synthesize-head-relation"), "{body}");
        }
        other => panic!("expected Completed for native impl_item query, got {other:?}"),
    }

    // Repair Hook v2: missing trailing wildcards on known EDB relation `(edge a b "imports")`
    // are safely padded to 5 columns and reported via `pad-relation-wildcards`.
    let padded_edge_call = ModelToolCall {
        id: ModelToolCallId::new("call-pad-edge-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "((answer path target) <-- (edge path target \"imports\") (file f \"rust\" path))"
                    .to_owned(),
            ),
        )]),
    };
    match handler.run_tool(padded_edge_call, owner.context()).await {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("src/caller.rs"), "{body}");
            assert!(body.contains("resident_kernel_entry"), "{body}");
            assert!(body.contains("pad-relation-wildcards"), "{body}");
        }
        other => panic!("expected Completed for padded 3-col edge query, got {other:?}"),
    }

    // Repair Hook v2 fail-closed diagnostic: unknown relation name rejects with POO
    // registry-backed `"did you mean: ..."` guidance instead of silently succeeding.
    let typo_rel_call = ModelToolCall {
        id: ModelToolCallId::new("call-typo-rel-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "((answer path name) <-- (file f \"rust\" path) (rust_function id f name line))"
                    .to_owned(),
            ),
        )]),
    };
    match handler.run_tool(typo_rel_call, owner.context()).await {
        ModelToolOutcome::Failed(err) => {
            assert!(err.contains("did you mean:"), "{err}");
            assert!(err.contains("rust_fn"), "{err}");
        }
        other => panic!("expected Failed with did-you-mean suggestion, got {other:?}"),
    }
}

#[tokio::test]
async fn workspace_search_auto_syncs_disk_edits_across_shared_slot_and_offer_governance() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(
        tmp.path().join("src/kernel.rs"),
        "pub fn initial_kernel_symbol() -> usize { 42 }\n",
    )
    .unwrap();

    let declarations = vec![
        ModelToolDeclaration::shell(ShellToolDeclarationRequest {
            name: ModelToolName::new("bash").unwrap(),
            description: "bounded shell".into(),
            argument: "command".into(),
            timeout_seconds: 10,
            output_limit: 4096,
        })
        .unwrap(),
        ModelToolDeclaration::search(SearchToolDeclarationRequest {
            name: ModelToolName::new("workspace_search").unwrap(),
            description: "resident search".into(),
            argument: "query".into(),
            timeout_seconds: 10,
            output_limit: 8192,
        })
        .unwrap(),
    ];
    assert_eq!(declarations.len(), 2);

    let warm = Arc::new(RwLock::new(WorkspaceSearchIndex::build_with_config(
        tmp.path(),
        WorkspaceIndexConfig::default(),
    )));
    let initial_generation = warm.read().unwrap().generation_digest().as_str().to_owned();
    let slot = SharedSearchIndexSlot::from_index(warm.clone());
    slot.attach_dsl_runner(shared_test_search_dsl_runner());

    let catalog = catalog_from_declarations_with_search(
        declarations.clone(),
        tmp.path().to_path_buf(),
        slot.clone(),
    )
    .unwrap();
    let offer = catalog.offer();
    let defs = offer.definitions();
    assert_eq!(defs[0].name.as_str(), "bash");
    assert_eq!(defs[1].name.as_str(), "workspace_search");
    assert_eq!(
        offer.governance(&ModelToolName::new("bash").unwrap()),
        Some(ModelToolGovernance::Ask)
    );
    assert_eq!(
        offer.governance(&ModelToolName::new("workspace_search").unwrap()),
        Some(ModelToolGovernance::Ask)
    );

    let handler =
        ResidentSearchToolHandler::new(tmp.path().to_path_buf(), declarations[1].clone(), slot);
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));

    // Mutate workspace on disk after warm-up (simulating a committed bash tool edit or external save).
    std::fs::write(
        tmp.path().join("src/fresh_caller.rs"),
        "use crate::kernel::initial_kernel_symbol;\npub fn live_synced_symbol() -> usize { initial_kernel_symbol() }\n",
    )
    .unwrap();

    let sym_call = ModelToolCall {
        id: ModelToolCallId::new("call-sync-sym").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "(ascent-fragment (relation answer ((path string?) (line integer?))) ((answer path line) <-- (file f \"rust\" path) (rust_fn id f \"live_synced_symbol\" line)))"
                    .to_owned(),
            ),
        )]),
    };
    let sym_outcome = handler.run_tool(sym_call, owner.context()).await;
    let body = match sym_outcome {
        ModelToolOutcome::Completed(body) => body,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert!(body.contains("src/fresh_caller.rs"), "{body}");
    assert!(body.contains("2"), "{body}");

    // Verify shared TUI Helm index handle saw the exact same generation bump and persisted snapshot.
    let updated_generation = warm.read().unwrap().generation_digest().as_str().to_owned();
    assert_ne!(initial_generation, updated_generation);
    assert!(body.contains(&updated_generation), "{body}");
    assert!(
        default_workspace_snapshot_path(tmp.path()).exists(),
        "expected incremental sync to persist cached mmap snapshot"
    );

    // Verify real upstream `:gerbil-ascent` import edge query immediately resolves the newly added importer.
    let graph_call = ModelToolCall {
        id: ModelToolCallId::new("call-sync-graph").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(
                "(ascent-fragment (relation answer ((path string?) (target string?))) ((answer path target) <-- (edge path target \"imports\" _ _) (file f \"rust\" path)))"
                    .to_owned(),
            ),
        )]),
    };
    match handler.run_tool(graph_call, owner.context()).await {
        ModelToolOutcome::Completed(graph_body) => {
            assert!(graph_body.contains("src/fresh_caller.rs"), "{graph_body}");
            assert!(graph_body.contains("initial_kernel_symbol"), "{graph_body}");
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sixteen_concurrent_agents_mixed_l0_l1_l2_scheduler_zero_deadlock_and_single_flight_dedup()
{
    use marlin_agent_tooling::{AsyncSearchDslRunner, SearchQueryTier};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

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
            marlin_agent_search_engine::WorkspaceEdbEdgeFact {
                src: "src/app.rs#fn:dispatch_search_batch".to_owned(),
                dst: "local_init".to_owned(),
                relation: "calls".to_owned(),
                path: "src/app.rs".to_owned(),
                line: 3,
            },
            marlin_agent_search_engine::WorkspaceEdbEdgeFact {
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
    let slot = SharedSearchIndexSlot::with_sync_debounce(Some(warm), Duration::from_secs(5));

    // Async priority runner tracking actual Gerbil VM invocations.
    let vm_calls = Arc::new(AtomicUsize::new(0));
    let l1_calls = Arc::new(AtomicUsize::new(0));
    let l2_calls = Arc::new(AtomicUsize::new(0));
    let vm_calls_ref = Arc::clone(&vm_calls);
    let l1_ref = Arc::clone(&l1_calls);
    let l2_ref = Arc::clone(&l2_calls);
    let sync_runner = shared_test_search_dsl_runner();
    let async_runner: AsyncSearchDslRunner = Arc::new(move |shared_index, query, limit, tier| {
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
            tokio::task::spawn_blocking(move || runner(shared_index, &query, limit))
                .await
                .map_err(|_| SearchPlanProjectionError::Invalid("join error"))?
        })
    });
    slot.attach_async_dsl_runner(async_runner);

    let decl = ModelToolDeclaration::search(SearchToolDeclarationRequest {
        name: ModelToolName::new("workspace_search").unwrap(),
        description: "Resident search".into(),
        argument: "query".into(),
        timeout_seconds: 15,
        output_limit: 8192,
    })
    .unwrap();
    let handler = Arc::new(ResidentSearchToolHandler::new(
        tmp.path().to_path_buf(),
        decl,
        slot.clone(),
    ));
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(32));

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
    let metrics = slot.scheduler_metrics();
    // 16 concurrent agents split across 3 unique queries => at most 1 VM call for L1 + 1 VM call for L2!
    assert_eq!(metrics.l0_brute_queries, 6);
    assert_eq!(metrics.l1_ascent_queries, 5);
    assert_eq!(metrics.l2_dusa_queries, 5);
    assert_eq!(
        vm_calls.load(Ordering::Relaxed),
        2,
        "10 concurrent L1+L2 agents across 2 unique queries must coalesce into exactly 2 Gerbil VM calls! metrics={metrics:?}"
    );
    assert_eq!(
        metrics.disk_sync_executed, 1,
        "16 agents must coalesce into 1 disk sync"
    );
    assert_eq!(metrics.disk_sync_coalesced_or_debounced, 15);
    assert_eq!(
        metrics.cache_hits + metrics.single_flight_coalesced,
        13,
        "16 queries over 3 unique keys => 13 cache/single-flight hits, got {metrics:?}"
    );

    println!(
        "\n=== 16-AGENT CONCURRENT MIXED SEARCH BENCHMARK (L0+L1+L2) ===\n\
         Total wall time (16 agents)  : {total_us}us\n\
         L0 Brute-Force (6 agents)    : latencies_us={l0_latencies:?} (0 Gerbil VM calls)\n\
         L1 Case A Ascent (5 agents)  : latencies_us={l1_latencies:?} ({} actual VM call)\n\
         L2 Case B Dusa (5 agents)    : latencies_us={l2_latencies:?} ({} actual VM call)\n\
         Single-flight + cache hits   : {} / 16 (coalesced={}, cache={})\n\
         Disk syncs executed          : {} / 16 (coalesced/debounced={})\n\
         =============================================================",
        l1_calls.load(Ordering::Relaxed),
        l2_calls.load(Ordering::Relaxed),
        metrics.cache_hits + metrics.single_flight_coalesced,
        metrics.single_flight_coalesced,
        metrics.cache_hits,
        metrics.disk_sync_executed,
        metrics.disk_sync_coalesced_or_debounced,
    );
}

#[tokio::test]
async fn search_gaps_regression_file_join_impl_trait_multiline_regex_coverage_and_validation() {
    use marlin_agent_search_engine::{
        WorkspaceEdbAttr, WorkspaceEdbEdgeFact, WorkspaceEdbEntityFact, WorkspaceIndexConfig,
        WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex,
    };

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

    let slot = SharedSearchIndexSlot::with_sync_debounce(
        Some(Arc::new(RwLock::new(idx))),
        std::time::Duration::from_secs(60),
    );
    slot.attach_dsl_runner(shared_test_search_dsl_runner());
    let decl = ModelToolDeclaration::search(SearchToolDeclarationRequest {
        name: ModelToolName::new("workspace_search").unwrap(),
        description: "Resident search".into(),
        argument: "query".into(),
        timeout_seconds: 15,
        output_limit: 8192,
    })
    .unwrap();
    let handler = ResidentSearchToolHandler::new(tmp.path().to_path_buf(), decl, slot);
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));

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
        ModelToolOutcome::Failed(err) => panic!("expected Completed for exact file path join, got Failed: {err}"),
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
        ModelToolOutcome::Failed(err) => panic!("expected Completed for reverse file join, got Failed: {err}"),
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
        ModelToolOutcome::Failed(err) => panic!("expected Completed for impl_trait, got Failed: {err}"),
    };
    assert!(impl_trait_out.contains("AotArchiveLinkReceipt"), "{impl_trait_out}");
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
    assert!(limited_out.contains("unwrap-limit-clause"), "{limited_out}");

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

    // Gap 6a: `re:^impl` matches line-anchored `impl ModelGateway ...` via native multiline mode (zero spurious repair tag)!
    let ModelToolOutcome::Completed(re_anchor) =
        run("gap-6a", "(brute-search \"re:^impl \" lang: \"rust\")").await
    else {
        panic!("expected Completed for re:^impl");
    };
    assert!(re_anchor.contains("src/archive.rs:4"), "{re_anchor}");
    assert!(!re_anchor.contains("enable-multiline-anchors"), "{re_anchor}");

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
    assert!(re_escapes.contains("AotArchiveLinkReceipt"), "{re_escapes}");
    assert!(re_escapes.contains("pub trait ModelGateway"), "{re_escapes}");
}
