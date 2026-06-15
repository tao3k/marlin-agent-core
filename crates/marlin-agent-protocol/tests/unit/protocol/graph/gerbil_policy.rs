use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID, GerbilLoopGraphContinuationCompileError,
    GerbilLoopGraphContinuationRequest, GerbilLoopGraphPolicyCompilationRequest,
    GraphLoopNextAction, GraphLoopStrategy, GraphLoopStrategyRuntime, GraphNativeAbiRequirement,
    GraphNativeSymbol, compile_gerbil_loop_graph_continuation, compile_gerbil_loop_graph_policy,
};

#[test]
fn gerbil_loop_graph_ir_compiles_into_graph_policy_proposal() {
    let compiled_graph = marlin_gerbil_ir::CompiledLoopGraph {
        graph_id: "gerbil-graph".to_string(),
        nodes: vec![
            marlin_gerbil_ir::LoopNodeSpec {
                id: "rank".to_string(),
                executor: "gerbil.rank".to_string(),
                config: BTreeMap::from([("mode".to_string(), "native".to_string())]),
            },
            marlin_gerbil_ir::LoopNodeSpec {
                id: "dispatch".to_string(),
                executor: "kernel.dispatch".to_string(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![marlin_gerbil_ir::LoopEdgeSpec {
            from: "rank".to_string(),
            to: "dispatch".to_string(),
            condition: Some("always".to_string()),
        }],
    };

    let proposal = compile_gerbil_loop_graph_policy(
        GerbilLoopGraphPolicyCompilationRequest::new(
            GraphLoopStrategy::native_gerbil("gerbil-loop-ranker", "v1"),
            compiled_graph,
            "sha256:gerbil-input",
            "sha256:gerbil-output",
        )
        .with_native_abi_requirement(native_policy_abi_requirement())
        .with_diagnostic("gerbil_ir=compiled"),
    )
    .expect("Gerbil loop graph IR should compile into graph policy proposal");

    assert_eq!(
        proposal.strategy.runtime,
        GraphLoopStrategyRuntime::NativeGerbil
    );
    assert_eq!(proposal.proposed_graph.graph_id, "gerbil-graph");
    assert_eq!(proposal.proposed_graph.nodes[0].executor, "gerbil.rank");
    assert_eq!(
        proposal.proposed_graph.edges[0].condition.as_deref(),
        Some("always")
    );
    assert_eq!(
        proposal
            .native_abi
            .as_ref()
            .expect("native abi")
            .required_symbols,
        vec![
            GraphNativeSymbol::new("marlin_graph_loop_rank"),
            GraphNativeSymbol::new("marlin_graph_loop_select"),
        ]
    );
    assert_eq!(proposal.diagnostics, vec!["gerbil_ir=compiled"]);
    assert!(proposal.validate().is_accepted());
}

#[test]
fn gerbil_loop_graph_continuation_compiles_into_next_action() {
    let action = compile_gerbil_loop_graph_continuation(
        GerbilLoopGraphContinuationRequest::continue_with_graph(valid_compiled_gerbil_graph()),
    )
    .expect("Gerbil continuation should compile into a controller next action");

    let GraphLoopNextAction::ContinueWithGraph(graph) = action else {
        panic!("expected continuation graph action");
    };
    assert_eq!(graph.graph_id, "gerbil-continuation-graph");
    assert_eq!(graph.nodes[0].executor, "gerbil.rank");
}

#[test]
fn gerbil_loop_graph_continuation_rejects_schema_mismatch() {
    let mut request = GerbilLoopGraphContinuationRequest::stop_completed();
    request.schema_id = "marlin.agent.gerbil_loop_graph_continuation.v0".to_owned();

    let error = compile_gerbil_loop_graph_continuation(request)
        .expect_err("schema mismatch should not compile");

    assert_eq!(
        error,
        GerbilLoopGraphContinuationCompileError::SchemaMismatch {
            expected: GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID.to_owned(),
            actual: "marlin.agent.gerbil_loop_graph_continuation.v0".to_owned(),
        }
    );
}

#[test]
fn gerbil_loop_graph_continuation_rejects_unpreserved_diagnostics() {
    let error = compile_gerbil_loop_graph_continuation(
        GerbilLoopGraphContinuationRequest::stop_completed().with_diagnostic("poo=terminal"),
    )
    .expect_err("diagnostic-bearing continuation requires a receipt before execution");

    assert_eq!(
        error,
        GerbilLoopGraphContinuationCompileError::DiagnosticRejected(vec![
            "poo=terminal".to_owned()
        ])
    );
}

#[test]
fn gerbil_loop_graph_continuation_terminal_actions_remain_typed() {
    let stop_completed = GerbilLoopGraphContinuationRequest::stop_completed();
    assert!(stop_completed.has_current_schema());

    let encoded = serde_json::to_value(&stop_completed).expect("continuation action serializes");
    assert_eq!(
        encoded["schema_id"],
        GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID
    );
    assert_eq!(encoded["action"]["kind"], "stop_completed");

    assert_eq!(
        compile_gerbil_loop_graph_continuation(stop_completed).expect("stop completed compiles"),
        GraphLoopNextAction::StopCompleted
    );
    assert_eq!(
        compile_gerbil_loop_graph_continuation(
            GerbilLoopGraphContinuationRequest::escalate_to_human("review continuation policy"),
        )
        .expect("human escalation compiles"),
        GraphLoopNextAction::EscalateToHuman {
            reason: "review continuation policy".to_string(),
        }
    );
}

fn native_policy_abi_requirement() -> GraphNativeAbiRequirement {
    GraphNativeAbiRequirement::new("marlin.graph-loop.native", 1)
        .with_required_symbols(["marlin_graph_loop_rank", "marlin_graph_loop_select"])
}

fn valid_compiled_gerbil_graph() -> marlin_gerbil_ir::CompiledLoopGraph {
    marlin_gerbil_ir::CompiledLoopGraph {
        graph_id: "gerbil-continuation-graph".to_string(),
        nodes: vec![marlin_gerbil_ir::LoopNodeSpec {
            id: "rank".to_string(),
            executor: "gerbil.rank".to_string(),
            config: BTreeMap::from([("mode".to_string(), "native".to_string())]),
        }],
        edges: Vec::new(),
    }
}
