use marlin_agent_core::{
    GraphLoopExecutionStatus, MODEL_ROUTE_ADMISSION_SCHEMA_ID, ModelContextForkMode,
    ModelRouteAdmissionMode, ModelSessionLifecycle, RuntimeHomeSource, SmokeLlmMode,
    SmokeRuntimeReceipt, SmokeRuntimeScenario, run_marlin_cli_from_args,
};
use tempfile::tempdir;

#[test]
fn debug_cli_smoke_runtime_executes_builtin_adapter_scenario() {
    let result = run_marlin_cli_from_args(["smoke", "runtime", "--scenario", "builtin-adapters"]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: SmokeRuntimeReceipt =
        serde_json::from_str(&result.stdout).expect("runtime smoke receipt");
    assert_eq!(receipt.scenario, SmokeRuntimeScenario::BuiltinAdapters);
    assert_eq!(receipt.llm_mode, SmokeLlmMode::NoLiveLlm);
    assert!(receipt.passed);
    assert_eq!(receipt.terminal_status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.node_count, 3);
    assert_eq!(receipt.visited_nodes, vec!["tool", "provider", "subagent"]);
    assert_eq!(receipt.tool_spawn_count, 1);
    assert_eq!(receipt.provider_spawn_count, 1);
    assert_eq!(receipt.subagent_spawn_count, 1);
    assert_eq!(receipt.process_spawn_count, 0);
    assert!(receipt.execution_result.is_some());
    assert!(receipt.state_home.is_none());
}

#[test]
fn debug_cli_smoke_runtime_process_command_fanout_reports_spawn_diagnostics() {
    let result = run_marlin_cli_from_args([
        "smoke",
        "runtime",
        "--scenario",
        "process-command-fanout",
        "--node-count",
        "3",
        "--command",
        "/bin/echo",
        "--arg",
        "fanout",
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: SmokeRuntimeReceipt =
        serde_json::from_str(&result.stdout).expect("runtime smoke receipt");
    assert_eq!(receipt.scenario, SmokeRuntimeScenario::ProcessCommandFanout);
    assert_eq!(receipt.llm_mode, SmokeLlmMode::NoLiveLlm);
    assert!(receipt.passed);
    assert_eq!(receipt.terminal_status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.node_count, 3);
    assert_eq!(
        receipt.visited_nodes,
        vec!["process-0", "process-1", "process-2"]
    );
    assert_eq!(receipt.tool_spawn_count, 3);
    assert_eq!(receipt.provider_spawn_count, 0);
    assert_eq!(receipt.subagent_spawn_count, 0);
    assert_eq!(receipt.process_spawn_count, 3);
    assert_eq!(receipt.node_receipt_count, 3);
    assert_eq!(receipt.completed_node_receipt_count, 3);
    assert_eq!(receipt.failed_node_receipt_count, 0);
    assert!(receipt.execution_result.is_some());
    assert!(receipt.state_home.is_none());
    assert!(
        receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "process-command.stdout:marlin-smoke fanout")
    );
    assert!(
        receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "process-command.node:process-2")
    );
}

#[test]
fn debug_cli_smoke_runtime_model_route_dry_run_reports_typed_route_receipt() {
    let result =
        run_marlin_cli_from_args(["smoke", "runtime", "--scenario", "model-route-dry-run"]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: SmokeRuntimeReceipt =
        serde_json::from_str(&result.stdout).expect("runtime smoke receipt");
    assert_eq!(receipt.scenario, SmokeRuntimeScenario::ModelRouteDryRun);
    assert_eq!(receipt.llm_mode, SmokeLlmMode::NoLiveLlm);
    assert!(receipt.passed);
    assert_eq!(receipt.terminal_status, GraphLoopExecutionStatus::Completed);
    assert_eq!(receipt.node_count, 0);
    assert_eq!(receipt.provider_spawn_count, 0);
    assert_eq!(receipt.subagent_spawn_count, 0);
    assert_eq!(receipt.process_spawn_count, 0);
    assert!(receipt.execution_result.is_none());
    assert!(receipt.state_home.is_none());

    let dry_run = receipt.model_route.expect("model route dry-run summary");
    assert_eq!(dry_run.rule_count, 1);
    assert_eq!(dry_run.request.task_kind.as_str(), "chat");
    assert_eq!(
        dry_run.request.route_request.command_line(),
        "marlin chat --dry-run"
    );
    let response = dry_run.response;
    assert_eq!(response.schema_id, MODEL_ROUTE_ADMISSION_SCHEMA_ID);
    assert_eq!(
        response.model_routing_mode,
        ModelRouteAdmissionMode::Deterministic
    );
    assert_eq!(
        response.intent.evidence_profile.as_str(),
        "smoke-model-route-dry-run"
    );
    assert_eq!(response.decision.endpoint.provider.as_str(), "openai");
    assert_eq!(response.decision.endpoint.model.as_str(), "gpt-5-mini");
    assert_eq!(
        response.decision.receipt.litellm_model_id.as_str(),
        "openai/gpt-5-mini"
    );
    assert_eq!(
        response.decision.receipt.rule_id.as_str(),
        "smoke-root-chat"
    );
    assert_eq!(
        response
            .decision
            .receipt
            .agent_scope
            .expect("agent scope")
            .as_str(),
        "RootAgent"
    );
    assert_eq!(
        response.decision.receipt.context_fork,
        ModelContextForkMode::ForkSnapshot
    );
    assert_eq!(
        response.decision.receipt.session_lifecycle,
        ModelSessionLifecycle::Persistent {
            key: "smoke-root-chat-session".into()
        }
    );
    assert_eq!(
        response
            .decision
            .receipt
            .requested_session_id
            .expect("requested session id")
            .as_str(),
        "smoke-route-session"
    );
    assert!(
        response
            .decision
            .receipt
            .matched_globs
            .contains(&"command_kind:chat".to_owned())
    );
    assert!(
        response
            .decision
            .receipt
            .matched_globs
            .contains(&"agent_scope:RootAgent".to_owned())
    );
    assert!(
        response
            .decision
            .receipt
            .matched_globs
            .contains(&"workspace:smoke://project/marlin-agent-core".to_owned())
    );
}

#[test]
fn debug_cli_smoke_runtime_state_home_env_prefers_marlin_home_over_home() {
    let dir = tempdir().expect("tempdir");
    let marlin_home = dir.path().join("custom-marlin-home");
    let host_home = dir.path().join("host-home");

    let result = run_marlin_cli_from_args([
        "smoke",
        "runtime",
        "--scenario",
        "state-home-env",
        "--marlin-home",
        marlin_home.to_str().expect("utf8 marlin home"),
        "--host-home",
        host_home.to_str().expect("utf8 host home"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: SmokeRuntimeReceipt =
        serde_json::from_str(&result.stdout).expect("runtime smoke receipt");
    assert_eq!(receipt.scenario, SmokeRuntimeScenario::StateHomeEnv);
    assert_eq!(receipt.llm_mode, SmokeLlmMode::NoLiveLlm);
    assert!(receipt.passed);
    assert_eq!(receipt.node_count, 0);
    assert_eq!(receipt.terminal_status, GraphLoopExecutionStatus::Completed);
    assert!(receipt.execution_result.is_none());
    let state_home = receipt.state_home.expect("state home summary");
    assert_eq!(state_home.home, marlin_home);
    assert_eq!(state_home.source, RuntimeHomeSource::Custom);
    assert_eq!(state_home.directory_count, 10);
    assert_eq!(
        state_home.receipt_path,
        state_home
            .home
            .join("receipts/marlin-smoke-state-home-env.json")
    );
    assert_eq!(
        state_home.graph_cache_path,
        state_home
            .home
            .join("cache/graph/marlin-smoke-state-home-env.json")
    );
}

#[test]
fn debug_cli_smoke_runtime_state_home_env_uses_home_default_when_marlin_home_absent() {
    let dir = tempdir().expect("tempdir");
    let host_home = dir.path().join("host-home");

    let result = run_marlin_cli_from_args([
        "smoke",
        "runtime",
        "--scenario",
        "state-home-env",
        "--host-home",
        host_home.to_str().expect("utf8 host home"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: SmokeRuntimeReceipt =
        serde_json::from_str(&result.stdout).expect("runtime smoke receipt");
    assert_eq!(receipt.scenario, SmokeRuntimeScenario::StateHomeEnv);
    assert_eq!(receipt.llm_mode, SmokeLlmMode::NoLiveLlm);
    assert!(receipt.passed);
    assert!(receipt.execution_result.is_none());
    let state_home = receipt.state_home.expect("state home summary");
    assert_eq!(state_home.home, host_home.join(".marlin"));
    assert_eq!(state_home.source, RuntimeHomeSource::Default);
    assert_eq!(
        state_home.session_path,
        state_home
            .home
            .join("sessions/marlin-smoke-state-home-env.json")
    );
    assert_eq!(
        state_home.memory_shard_path,
        state_home
            .home
            .join("memory/marlin-smoke-state-home-env.json")
    );
}
