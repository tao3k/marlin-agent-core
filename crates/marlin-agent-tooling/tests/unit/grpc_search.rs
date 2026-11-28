// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[cfg(feature = "grpc")]
mod grpc_tests {
    use std::sync::{Arc, RwLock};

    use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
    use marlin_agent_runtime::{
        ModelToolDeclaration, ModelToolOutcome, SearchToolDeclarationRequest, TokioAgentRuntime,
        TokioAgentRuntimeBuildRequest, ToolRuntime,
    };
    use marlin_agent_search_engine::{
        SearchDslExecutionRequest, SearchDslSessionExecutionRequest, SearchDslSessionId,
        SearchDslSessionRunner, SearchPlanProjectionError, WorkspaceEdbEntityFact,
        WorkspaceIndexConfig, WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex,
        default_marlin_scheme_dir, execute_package_search_dsl_session_structured,
    };
    use marlin_agent_tooling::grpc_search::{
        GetToolSchemaRequest, GrpcSearchToolHandler, LoopbackSearchGrpcServer,
        SearchGatewayService, SearchStepRequest, tool_schema_entries_from_declarations,
    };

    fn test_session_runner() -> SearchDslSessionRunner {
        Arc::new(|idx, query, output_limit, session_id| {
            let scheme_dir = default_marlin_scheme_dir()
                .ok_or(SearchPlanProjectionError::Invalid("missing scheme dir"))?
                .to_string_lossy()
                .into_owned();
            let query = query.to_owned();
            let sid = session_id.clone();
            crate::projection::with_shared_test_gerbil_vm(move |rt| {
                let guard = idx
                    .read()
                    .map_err(|_| SearchPlanProjectionError::Invalid("index lock poisoned"))?;
                let req = SearchDslSessionExecutionRequest {
                    execution: SearchDslExecutionRequest {
                        index: &guard,
                        query: &query,
                        output_limit,
                    },
                    session_id: &sid,
                };
                execute_package_search_dsl_session_structured(rt, &scheme_dir, &req)
            })
        })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tonic_grpc_search_gateway_progressive_evidence_chain_and_self_teaching_loop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bridge_dir = dir
            .path()
            .join("crates/marlin-agent-search-engine/src/edb_bridge");
        let scheme_cabi = dir.path().join("scheme/c-abi");
        std::fs::create_dir_all(&bridge_dir).unwrap();
        std::fs::create_dir_all(&scheme_cabi).unwrap();
        std::fs::write(
            bridge_dir.join("runner.rs"),
            "pub struct SearchDslExecutionRequest;\npub fn execute_dsl() {}\n",
        )
        .unwrap();
        std::fs::write(bridge_dir.join("codec.rs"), "pub fn decode_rows() {}\n").unwrap();
        std::fs::write(
            scheme_cabi.join("search-dsl.ss"),
            "(def (marlin-search-dsl-eval-edb input) input)\n",
        )
        .unwrap();

        let mut index =
            WorkspaceSearchIndex::build_with_config(dir.path(), WorkspaceIndexConfig::default());
        index.ingest_open_edb_facts(WorkspaceOpenEdbIngestBatch {
            language_id: Some("rust".to_owned()),
            provider_id: Some("test-rust".to_owned()),
            entities: vec![WorkspaceEdbEntityFact {
                entity_id: "e-req".to_owned(),
                file_id: 0,
                language: "rust".to_owned(),
                kind: "struct".to_owned(),
                language_kind: "struct_item".to_owned(),
                name: "SearchDslExecutionRequest".to_owned(),
                path: "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs".to_owned(),
                line: 1,
                source: "test-rust".to_owned(),
                attrs: Vec::new(),
            }],
            ..Default::default()
        });

        let decl = ModelToolDeclaration::search(SearchToolDeclarationRequest {
            name: ModelToolName::new("structural_query").unwrap(),
            description: "gRPC structural search".to_owned(),
            argument: "query".to_owned(),
            timeout_seconds: 30,
            output_limit: 16384,
        })
        .unwrap();

        let (rt, _ev) =
            TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));
        let service = SearchGatewayService::new(
            Arc::new(RwLock::new(index)),
            tool_schema_entries_from_declarations(std::slice::from_ref(&decl)),
            test_session_runner(),
            rt.context(),
        );
        let server = LoopbackSearchGrpcServer::spawn(service, &rt).expect("spawn loopback gRPC");
        let mut client = server.client();

        // 1. GetToolSchema over real HTTP/2 tonic gRPC
        let schema = client
            .get_tool_schema(GetToolSchemaRequest {})
            .await
            .expect("GetToolSchema")
            .into_inner();
        assert_eq!(schema.tools.len(), 1);
        assert_eq!(schema.tools[0].name, "structural_query");
        assert!(schema.tools[0].read_only);

        // 2. Turn 1 Dusa placement: teaching_kit_active == true, workspace_layers populated,
        //    live Scheme evidence-chain.ss harvests 2 invariant M1 `verified_tier` claims!
        let t1 = client
            .search_step(SearchStepRequest {
                session_id: "sess-grpc-1".to_owned(),
                tool_name: "constraint_solve".to_owned(),
                query: "(solve-placement path-prefix: \"crates/marlin-agent-search-engine/src/edb_bridge/\" module-limit: auto choices: (resident provider) forbid-hot-path: provider state-budget: 64)".to_owned(),
                output_limit: 16384,
            })
            .await
            .expect("Turn 1 SearchStep")
            .into_inner();
        let f1 = t1.frontier.expect("Turn 1 frontier");
        assert_eq!(f1.step, 1);
        assert!(f1.teaching_kit_active);
        assert!(!f1.compact_mode);
        assert!(f1.workspace_layers.contains("crates/(marlin-agent-search-engine)"));
        assert!(f1.workspace_layers.contains("scheme/(c-abi)"));
        assert_eq!(f1.total_m1_claims, 2);
        assert_eq!(f1.newly_harvested, 2);
        assert_eq!(f1.epistemic_status, "unique-deduction");

        // 3. Turn 2 Ascent directly consumes Turn 1's M1 `verified_tier` ("越搜越强")
        //    and auto-unloads teaching kit (`compact_mode == true`, `workspace_layers == ""`).
        let t2 = client
            .search_step(SearchStepRequest {
                session_id: "sess-grpc-1".to_owned(),
                tool_name: "structural_query".to_owned(),
                query: "(ascent-fragment ((answer m lvl) <-- (verified_tier m lvl)))".to_owned(),
                output_limit: 16384,
            })
            .await
            .expect("Turn 2 SearchStep")
            .into_inner();
        let f2 = t2.frontier.expect("Turn 2 frontier");
        assert_eq!(f2.step, 2);
        assert!(!f2.teaching_kit_active);
        assert!(f2.compact_mode);
        assert!(f2.workspace_layers.is_empty());
        assert!(t2.rendered_text.contains("runner.rs"));
        assert!(t2.rendered_text.contains("resident"));
        assert!(f2.open_frontiers.iter().any(|p| p.contains("scheme/c-abi")));

        // 4. Turn 3 via thin `GrpcSearchToolHandler` closes `scheme/c-abi` -> converged-evidence!
        let sid = SearchDslSessionId::new("sess-grpc-1").unwrap();
        let session_cell = Arc::new(RwLock::new(sid.clone()));
        let handler = GrpcSearchToolHandler::new(decl, server.client(), session_cell);
        let ModelToolOutcome::Completed(t3_body) = handler
            .run_tool(
                ModelToolCall {
                    id: ModelToolCallId::new("grpc-t3").unwrap(),
                    name: ModelToolName::new("structural_query").unwrap(),
                    arguments: serde_json::Map::from_iter([(
                        "query".to_owned(),
                        serde_json::Value::String(
                            "(read-file \"scheme/c-abi/search-dsl.ss\" cursor: 1 max-lines: 40)"
                                .to_owned(),
                        ),
                    )]),
                },
                rt.context(),
            )
            .await
        else {
            panic!("expected Completed from GrpcSearchToolHandler");
        };
        assert!(t3_body.contains("compact_mode: true"), "{t3_body}");
        assert!(t3_body.contains("status: converged-evidence"), "{t3_body}");
        assert!(t3_body.contains("open_frontiers: none"), "{t3_body}");

        // 5. ResetSession clears server state and re-arms Turn-1 self-teaching!
        assert!(server.reset_session(&sid).await.expect("ResetSession"));
        let t4 = client
            .search_step(SearchStepRequest {
                session_id: "sess-grpc-1".to_owned(),
                tool_name: "workspace_search".to_owned(),
                query: "(read-file \"scheme/c-abi/search-dsl.ss\" cursor: 1 max-lines: 20)".to_owned(),
                output_limit: 16384,
            })
            .await
            .expect("Post-reset SearchStep")
            .into_inner();
        let f4 = t4.frontier.expect("Post-reset frontier");
        assert_eq!(f4.step, 1);
        assert!(f4.teaching_kit_active);
        assert!(!f4.workspace_layers.is_empty());
    }
}
