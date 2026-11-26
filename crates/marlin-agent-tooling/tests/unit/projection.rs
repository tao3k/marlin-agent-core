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
            assert!(err.contains(":gerbil-ascent"), "{err}");
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
            assert!(body.contains("synthesize-head-relation"), "{body}");
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
