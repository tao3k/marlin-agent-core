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
    WorkspaceIndexConfig, WorkspaceSearchIndex, default_workspace_snapshot_path,
};
use marlin_agent_tooling::{
    ResidentSearchToolHandler, SharedSearchIndexSlot, catalog_from_declarations_with_search,
    decode_tooling,
};
use std::sync::{Arc, RwLock};

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
    let runtime = GerbilRuntime::initialize().unwrap();
    assert_eq!(
        decode_tooling(vector(
            &runtime,
            &[declaration(&runtime, "shell-one", "ask")]
        ))
        .unwrap()
        .len(),
        1
    );
    assert!(
        decode_tooling(vector(
            &runtime,
            &[declaration(&runtime, "shell-one", "allow")]
        ))
        .is_err()
    );
    assert!(
        decode_tooling(vector(
            &runtime,
            &[
                declaration(&runtime, "shell-one", "ask"),
                declaration(&runtime, "shell-one", "ask"),
            ],
        ))
        .is_err()
    );

    let search_decl = vector(
        &runtime,
        &[
            symbol(&runtime, "workspace_search"),
            runtime
                .string_from_utf8("resident search")
                .unwrap()
                .into_object(),
            symbol(&runtime, "search"),
            symbol(&runtime, "ask"),
            symbol(&runtime, "query"),
            runtime.scalar_object(SchemeScalar::Fixnum(10)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(4096)).unwrap(),
        ],
    );
    let decoded = decode_tooling(vector(&runtime, &[search_decl])).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].capability(), ModelToolCapability::Search);
    let catalog = catalog_from_declarations_with_search(
        decoded,
        std::path::PathBuf::from("."),
        SharedSearchIndexSlot::empty(),
    )
    .unwrap();
    assert_eq!(catalog.offer().definitions().len(), 1);
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
    let slot = SharedSearchIndexSlot::from_index(warm);
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
            serde_json::Value::String("@lang:rust @sym:resident_kernel_entry".to_owned()),
        )]),
    };
    let outcome = handler.run_tool(sym_call, owner.context()).await;
    match outcome {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("generation: blake3-256:"), "{body}");
            assert!(body.contains("src/kernel.rs:1"), "{body}");
            assert!(body.contains("sym=(fn:resident_kernel_entry)"), "{body}");
        }
        other => panic!("expected Completed, got {other:?}"),
    }

    let graph_call = ModelToolCall {
        id: ModelToolCallId::new("call-graph-1").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String("@graph:resident_kernel_entry #caller".to_owned()),
        )]),
    };
    let graph_outcome = handler.run_tool(graph_call, owner.context()).await;
    match graph_outcome {
        ModelToolOutcome::Completed(body) => {
            assert!(body.contains("src/caller.rs"), "{body}");
            assert!(body.contains("graph=("), "{body}");
            assert!(
                !body.contains("src/kernel.rs"),
                "local #caller filter should prune kernel.rs: {body}"
            );
        }
        other => panic!("expected Completed, got {other:?}"),
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
            serde_json::Value::String("@sym:live_synced_symbol".to_owned()),
        )]),
    };
    let sym_outcome = handler.run_tool(sym_call, owner.context()).await;
    let body = match sym_outcome {
        ModelToolOutcome::Completed(body) => body,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert!(body.contains("src/fresh_caller.rs:2"), "{body}");
    assert!(body.contains("sym=(fn:live_synced_symbol)"), "{body}");

    // Verify shared TUI Helm index handle saw the exact same generation bump and persisted snapshot.
    let updated_generation = warm.read().unwrap().generation_digest().as_str().to_owned();
    assert_ne!(initial_generation, updated_generation);
    assert!(body.contains(&updated_generation), "{body}");
    assert!(
        default_workspace_snapshot_path(tmp.path()).exists(),
        "expected incremental sync to persist cached mmap snapshot"
    );

    // Verify 1-hop graph query immediately resolves the newly added importer.
    let graph_call = ModelToolCall {
        id: ModelToolCallId::new("call-sync-graph").unwrap(),
        name: ModelToolName::new("workspace_search").unwrap(),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String("@graph:initial_kernel_symbol #fresh_caller".to_owned()),
        )]),
    };
    match handler.run_tool(graph_call, owner.context()).await {
        ModelToolOutcome::Completed(graph_body) => {
            assert!(graph_body.contains("src/fresh_caller.rs"), "{graph_body}");
            assert!(graph_body.contains("graph=("), "{graph_body}");
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}
