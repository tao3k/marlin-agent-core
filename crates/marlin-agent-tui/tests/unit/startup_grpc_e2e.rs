// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real AOT-linked Gerbil Scheme VM + HTTP/2 `tonic` gRPC `SearchGatewayService` +
//! SM11 `LinkedSearchDslMailbox` end-to-end integration test exercising the exact
//! `marlin-agent-tui` startup & event-loop code path across 5 turns (including
//! L0 Recency-Peak Turn-1 self-teaching, Turn-2 `compact_mode: true`, Turn-3 L2
//! Dusa M1 `verified_tier` harvesting, Turn-4 cross-turn M1 consumption, and
//! Turn-5 `M-x session` session-switch isolation).

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCapability, ModelToolOutcome, RuntimeContext, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, ToolRuntime,
};
use marlin_agent_search_engine::{WorkspaceIndexConfig, WorkspaceSearchIndex};
use marlin_agent_tooling::GrpcSearchToolHandler;
use marlin_agent_tooling::grpc_search::GetToolSchemaRequest;

use super::{ToolCatalogRequest, build_tool_catalog, sync_active_search_session};
use crate::config::SessionConfig;
use crate::linked_ui::{self, LinkedSearchDslMailbox};

fn drive_tui_grpc_tool_call(
    rt: &tokio::runtime::Runtime,
    scheme_runtime: &gerbil_scheme::GerbilRuntime,
    mailbox: &mut LinkedSearchDslMailbox,
    handler: &GrpcSearchToolHandler,
    ctx: RuntimeContext,
    call_id: &str,
    query: &str,
) -> String {
    let call = ModelToolCall {
        id: ModelToolCallId::new(call_id).expect("call id"),
        name: ModelToolName::new("workspace_search").expect("tool name"),
        arguments: serde_json::Map::from_iter([(
            "query".to_owned(),
            serde_json::Value::String(query.to_owned()),
        )]),
    };
    let handler = handler.clone();
    let task = rt.spawn(async move { handler.run_tool(call, ctx).await });
    let deadline = Instant::now() + Duration::from_secs(30);
    while !task.is_finished() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for TUI HTTP/2 gRPC tool call {call_id}"
        );
        let _ = mailbox.drain_budgeted(scheme_runtime, Duration::from_millis(8));
        std::thread::sleep(Duration::from_millis(2));
    }
    match rt.block_on(task).expect("join TUI gRPC tool task") {
        ModelToolOutcome::Completed(body) => body,
        ModelToolOutcome::Failed(err) => panic!("TUI gRPC tool call {call_id} failed: {err}"),
    }
}

#[test]
fn tui_startup_grpc_gateway_mailbox_m1_evidence_chain_and_session_switch_e2e() {
    let ws = tempfile::tempdir().expect("temp workspace");
    let src_dir = ws
        .path()
        .join("crates/marlin-agent-search-engine/src/edb_bridge");
    let scheme_dir = ws.path().join("scheme/search-dsl");
    std::fs::create_dir_all(&src_dir).expect("mkdir src");
    std::fs::create_dir_all(&scheme_dir).expect("mkdir scheme");
    std::fs::write(
        src_dir.join("app.rs"),
        "pub fn local_init() -> usize { 1 }\n\
         pub fn run_app() -> usize { local_init() + parse_payload() }\n",
    )
    .expect("write app.rs");
    std::fs::write(
        src_dir.join("parser_a.rs"),
        "pub fn parse_payload() -> usize { 10 }\n",
    )
    .expect("write parser_a.rs");
    std::fs::write(scheme_dir.join("funcs.ss"), "(def (eval-query x) x)\n")
        .expect("write funcs.ss");

    let scheme_runtime = linked_ui::initialize().expect("initialize AOT-linked TUI Gerbil VM");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(16));
    let (search_dsl_runner, mut mailbox) = linked_ui::search_dsl_async_channel();

    let session_cfg = SessionConfig {
        path: ws.path().join("session.db"),
        project: "tui-grpc-e2e".to_owned(),
        session: "tui-e2e-session-a".to_owned(),
        agent: "marlin".to_owned(),
    };
    let bound = build_tool_catalog(ToolCatalogRequest {
        scheme_runtime: &scheme_runtime,
        directory: ws.path(),
        effective_session: Some(&session_cfg),
        search_dsl_runner,
        runtime_handle: rt.handle(),
        runtime_owner: &owner,
    })
    .expect("build_tool_catalog over loopback HTTP/2 tonic gRPC");

    // 1. Verify unified `workspace_search` schema advertised by TUI & GetToolSchema gRPC RPC.
    let catalog = bound.catalog.as_ref().expect("bound tool catalog");
    let defs = catalog.offer().definitions();
    let ws_def = defs
        .iter()
        .find(|d| d.name.as_str() == "workspace_search")
        .expect("workspace_search definition");
    assert!(ws_def.description.contains("(brute-search ...)"));
    assert!(ws_def.description.contains("(ascent-fragment ...)"));
    assert!(ws_def.description.contains("(search-choice-program ...)"));
    assert!(ws_def.description.contains("(solve-resolve "));
    assert!(ws_def.description.contains("(find-impact "));

    let grpc_server = bound.grpc_server.as_ref().expect("loopback gRPC server");
    let schema_resp = rt
        .block_on(async {
            let mut client = grpc_server.client();
            client.get_tool_schema(GetToolSchemaRequest {}).await
        })
        .expect("GetToolSchema over HTTP/2")
        .into_inner();
    assert_eq!(schema_resp.tools.len(), 1);
    assert_eq!(schema_resp.tools[0].name, "workspace_search");
    assert!(schema_resp.tools[0].read_only);

    // 2. Hot-attach warm index (matching `asp_supervisor.rs`) & bind Session A (matching `open_session_gateway`).
    let warm = Arc::new(RwLock::new(WorkspaceSearchIndex::build_with_config(
        ws.path(),
        WorkspaceIndexConfig::default(),
    )));
    bound.search_index_handle.attach(warm);
    sync_active_search_session(&bound.active_search_session, "tui-e2e-session-a");

    let search_decl = linked_ui::tooling_declarations(&scheme_runtime)
        .expect("project AOT tooling declarations")
        .into_iter()
        .find(|d| d.capability() == ModelToolCapability::Search)
        .expect("search tool declaration");
    let handler = GrpcSearchToolHandler::new(
        search_decl,
        grpc_server.client(),
        Arc::clone(&bound.active_search_session),
    );

    // Turn 1: L0 Brute-Force -> Turn-1 Recency-Peak `dsl_tips:` & `workspace_layers:`.
    let t1 = drive_tui_grpc_tool_call(
        &rt,
        &scheme_runtime,
        &mut mailbox,
        &handler,
        owner.context(),
        "tui-t1",
        "(brute-search \"parse_payload\" path-prefix: \"crates/\" lang: \"rust\")",
    );
    assert!(t1.contains("mode: brute-force"), "{t1}");
    assert!(t1.contains("app.rs"), "{t1}");
    assert!(
        t1.contains("[evidence_frontier |")
            && t1.contains("teaching_kit: active")
            && t1.contains("dsl_tips:")
            && t1.contains("workspace_layers:"),
        "Turn 1 missing Recency-Peak frontier: {t1}"
    );

    // Turn 2: L1 Ascent via SM11 LinkedSearchDslMailbox + AOT VM -> `compact_mode: true` auto-unloading!
    let t2 = drive_tui_grpc_tool_call(
        &rt,
        &scheme_runtime,
        &mut mailbox,
        &handler,
        owner.context(),
        "tui-t2",
        "(ascent-fragment ((answer path name line) <-- (file f \"rust\" path) (rust_fn _ f name line)))",
    );
    assert!(t2.contains("mode: ascent"), "{t2}");
    assert!(t2.contains("local_init") && t2.contains("parse_payload"), "{t2}");
    assert!(
        t2.contains("compact_mode: true") && !t2.contains("dsl_tips:"),
        "Turn 2 expected compact_mode auto-unloading: {t2}"
    );

    // Turn 3: L2 Dusa `(solve-placement ...)` harvests M1 `verified_tier` inside TUI's AOT Scheme VM!
    let t3 = drive_tui_grpc_tool_call(
        &rt,
        &scheme_runtime,
        &mut mailbox,
        &handler,
        owner.context(),
        "tui-t3",
        "(solve-placement path-prefix: \"crates/marlin-agent-search-engine/src/edb_bridge/\" module-limit: auto choices: (resident provider) forbid-hot-path: provider state-budget: 64)",
    );
    assert!(t3.contains("mode: choice"), "{t3}");
    assert!(t3.contains("compact_mode: true"), "{t3}");
    assert!(
        t3.contains("(+2)") && t3.contains("status: unique-deduction"),
        "Turn 3 expected 2 newly harvested M1 claims + unique-deduction: {t3}"
    );

    // Turn 4: L1 Ascent directly consumes Turn 3's M1 `verified_tier` facts ("越搜越强")!
    let t4 = drive_tui_grpc_tool_call(
        &rt,
        &scheme_runtime,
        &mut mailbox,
        &handler,
        owner.context(),
        "tui-t4",
        "(ascent-fragment ((answer m lvl) <-- (verified_tier m lvl)))",
    );
    assert!(t4.contains("mode: ascent"), "{t4}");
    assert!(
        t4.contains("app.rs") && t4.contains("parser_a.rs") && t4.contains("resident"),
        "Turn 4 expected M1 verified_tier facts from Turn 3: {t4}"
    );
    assert!(t4.contains("compact_mode: true"), "{t4}");

    // Turn 5: Simulate interactive `M-x session` switch -> fresh isolated Turn-1 Recency-Peak session!
    sync_active_search_session(&bound.active_search_session, "tui-e2e-session-b");
    let t5 = drive_tui_grpc_tool_call(
        &rt,
        &scheme_runtime,
        &mut mailbox,
        &handler,
        owner.context(),
        "tui-t5",
        "(file-outline \"crates/marlin-agent-search-engine/src/edb_bridge/app.rs\")",
    );
    assert!(t5.contains("mode: file-outline"), "{t5}");
    assert!(
        t5.contains("dsl_tips:") && t5.contains("workspace_layers:"),
        "switched session expected fresh Turn-1 Recency-Peak frontier: {t5}"
    );
}
