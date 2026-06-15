use std::path::PathBuf;

use marlin_agent_harness::{
    AgentHarness, AgentHarnessEvidence, AgentHarnessEvidenceKind, AgentHarnessGraphBuilder,
    AgentHarnessRuntime, AgentHarnessScenario,
};
use marlin_agent_kernel::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionStatus,
    GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation, TokioGraphLoopKernel,
};
use marlin_agent_protocol::AgentScenarioStep;
use marlin_agent_runtime::{RuntimeContext, RuntimeFuture, observability};
use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilCommandProfile, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gxi_program, gerbil_runtime_loadpath, resolve_gerbil_executable,
    write_gerbil_runtime_assets,
};
use tempfile::Builder;

const MARLIN_REQUIRE_REAL_GXI_ENV: &str = "MARLIN_REQUIRE_REAL_GXI";

#[tokio::test]
#[ignore = "requires a local Gerbil gxi executable"]
async fn harness_executes_graph_with_real_gxi_runtime_asset_plan() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = Builder::new()
        .prefix("marlin-agent-harness-real-gxi-")
        .tempdir()
        .expect("real gxi harness tempdir");
    let written_assets = write_gerbil_runtime_assets(root.path())
        .expect("write Gerbil runtime assets for harness live test");
    let command_profile =
        GerbilCommandProfile::marlin_runtime_module(gxi.to_string_lossy(), root.path());
    let loadpath = gerbil_runtime_loadpath(root.path());
    let adapter_path = root.path().join("src/marlin/adapter.ss");

    assert!(adapter_path.exists());
    assert_eq!(command_profile.args, vec![GERBIL_ADAPTER_MODULE.to_owned()]);
    assert_eq!(
        command_profile
            .env
            .get(GERBIL_LOADPATH_ENV)
            .map(String::as_str),
        Some(loadpath.to_string_lossy().as_ref())
    );

    let scenario = AgentHarnessScenario::new("real-gxi-runtime-asset-plan")
        .with_step(
            AgentScenarioStep::new("run")
                .expecting_event_topic(observability::TOPIC_KERNEL_EXECUTION)
                .expecting_span_name(observability::harness_execution_span_name()),
        )
        .expecting_evidence(AgentHarnessEvidenceKind::Runtime);
    let graph = AgentHarnessGraphBuilder::new("real-gxi-runtime-assets")
        .linear([("rank", "gerbil-rank"), ("dispatch", "kernel-dispatch")])
        .build();
    let request = GraphLoopExecutionRequest::new("real-gxi-harness-run", graph)
        .with_budget(GraphLoopExecutionBudget::max_node_executions(2));
    let kernel = TokioGraphLoopKernel::new("real-gxi-harness-run", "real-gxi-runtime-assets")
        .with_executor("gerbil-rank", RealGxiHarnessExecutor)
        .with_executor("kernel-dispatch", RealGxiHarnessExecutor);
    let mut harness = AgentHarnessRuntime::new(64);
    harness.record_evidence(
        AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Runtime, "real-gxi-runtime-assets")
            .with_detail(format!(
                "gxi={} adapter_module={} loadpath={} written_asset_count={}",
                gxi.display(),
                GERBIL_ADAPTER_MODULE,
                loadpath.display(),
                written_assets.len()
            )),
    );

    let report = harness.execute_graph(&scenario, &kernel, request).await;
    let evaluated = AgentHarness::evaluate_execution_report(&scenario, &report);

    assert_eq!(
        report.result.status,
        GraphLoopExecutionStatus::Completed,
        "real gxi runtime asset diagnostics: {:?}",
        report.result.diagnostics
    );
    assert_eq!(report.result.visited_nodes, vec!["rank", "dispatch"]);
    assert!(report.has_event_topic(&observability::kernel_execution_topic()));
    assert!(report.has_span(&observability::harness_execution_span_name()));
    assert!(report.assertion.is_none());
    assert!(
        evaluated.is_success(),
        "real gxi harness contract failed: {:?}",
        evaluated.diagnostics
    );
}

fn local_gxi() -> Option<PathBuf> {
    let configured_gxi = default_gerbil_gxi_program();
    let Some(gxi) = resolve_gerbil_executable(&configured_gxi) else {
        let message = format!(
            "skipping real gxi harness test because {} is missing",
            configured_gxi.display()
        );
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!("{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or set {MARLIN_GERBIL_GXI_ENV}");
        }
        eprintln!("{message}");
        return None;
    };

    Some(gxi)
}

#[derive(Clone, Debug)]
struct RealGxiHarnessExecutor;

impl GraphNodeExecutor for RealGxiHarnessExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            GraphNodeExecutionReceipt::completed(invocation.node_id, invocation.executor)
        })
    }
}
