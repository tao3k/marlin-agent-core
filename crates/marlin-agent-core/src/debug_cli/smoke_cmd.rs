//! `marlin smoke ...` command implementations.

use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    CompiledModelRouteResolver, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphLoopExecutionStatus, GraphLoopKernel, LoopGraph, LoopNodeSpec, ModelCommandMatcher,
    ModelContextForkMode, ModelEndpoint, ModelRouteAdmissionRequest, ModelRouteAgentScope,
    ModelRouteRequest, ModelRouteRule, ModelSessionPolicy, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, TokioAgentRuntime, runtime::admit_model_route_with_resolver,
};

use super::{
    MarlinCliResult,
    args::ArgCursor,
    catalog::DebugExecutorCatalog,
    executor::debug_kernel,
    io::block_on,
    process_command::ProcessCommandBinding,
    receipts::{
        SmokeLlmMode, SmokeRuntimeModelRouteDryRun, SmokeRuntimeReceipt, SmokeRuntimeScenario,
        SmokeRuntimeStateHome,
    },
    smoke_usage,
};

const BUILTIN_ADAPTER_RUN_ID: &str = "marlin-smoke-builtin-adapters";
const MODEL_ROUTE_DRY_RUN_ID: &str = "marlin-smoke-model-route-dry-run";
const PROCESS_COMMAND_RUN_ID: &str = "marlin-smoke-process-command-fanout";
const PROCESS_COMMAND_EXECUTOR: &str = "smoke.process.command";
const STATE_HOME_ENV_RUN_ID: &str = "marlin-smoke-state-home-env";

pub(super) fn dispatch_smoke(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing smoke command\n{}", smoke_usage()));
    };

    match command.as_str() {
        "runtime" => {
            let options = SmokeRuntimeOptions::parse(cursor)?;
            let receipt = run_runtime_smoke(options)?;
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(smoke_usage())),
        unknown => Err(format!(
            "unknown smoke command `{unknown}`\n{}",
            smoke_usage()
        )),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SmokeScenarioOption {
    BuiltinAdapters,
    ModelRouteDryRun,
    ProcessCommandFanout,
    StateHomeEnv,
}

impl SmokeScenarioOption {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "builtin-adapters" => Ok(Self::BuiltinAdapters),
            "model-route-dry-run" => Ok(Self::ModelRouteDryRun),
            "process-command-fanout" => Ok(Self::ProcessCommandFanout),
            "state-home-env" => Ok(Self::StateHomeEnv),
            unknown => Err(format!(
                "unsupported smoke runtime scenario `{unknown}`; expected builtin-adapters, model-route-dry-run, process-command-fanout, or state-home-env"
            )),
        }
    }

    fn receipt_scenario(&self) -> SmokeRuntimeScenario {
        match self {
            Self::BuiltinAdapters => SmokeRuntimeScenario::BuiltinAdapters,
            Self::ModelRouteDryRun => SmokeRuntimeScenario::ModelRouteDryRun,
            Self::ProcessCommandFanout => SmokeRuntimeScenario::ProcessCommandFanout,
            Self::StateHomeEnv => SmokeRuntimeScenario::StateHomeEnv,
        }
    }
}

#[derive(Clone, Debug)]
struct SmokeRuntimeOptions {
    scenario: SmokeScenarioOption,
    node_count: usize,
    command: PathBuf,
    args: Vec<String>,
    marlin_home: Option<PathBuf>,
    host_home: Option<PathBuf>,
}

impl SmokeRuntimeOptions {
    fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut scenario = SmokeScenarioOption::BuiltinAdapters;
        let mut node_count = 3;
        let mut command = PathBuf::from("/bin/echo");
        let mut args = vec!["marlin-smoke".to_owned()];
        let mut marlin_home = None;
        let mut host_home = None;

        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--scenario" => {
                    let value = cursor
                        .next()
                        .ok_or_else(|| "--scenario requires a value".to_owned())?;
                    scenario = SmokeScenarioOption::parse(&value)?;
                }
                "--node-count" => {
                    let value = cursor
                        .next()
                        .ok_or_else(|| "--node-count requires a value".to_owned())?;
                    node_count = parse_positive_usize("--node-count", &value)?;
                }
                "--command" => {
                    let value = cursor
                        .next()
                        .ok_or_else(|| "--command requires a value".to_owned())?;
                    if value.trim().is_empty() {
                        return Err("--command requires a non-empty value".to_owned());
                    }
                    command = PathBuf::from(value);
                }
                "--marlin-home" => {
                    let value = cursor
                        .next()
                        .ok_or_else(|| "--marlin-home requires a value".to_owned())?;
                    if value.trim().is_empty() {
                        return Err("--marlin-home requires a non-empty value".to_owned());
                    }
                    marlin_home = Some(PathBuf::from(value));
                }
                "--host-home" => {
                    let value = cursor
                        .next()
                        .ok_or_else(|| "--host-home requires a value".to_owned())?;
                    if value.trim().is_empty() {
                        return Err("--host-home requires a non-empty value".to_owned());
                    }
                    host_home = Some(PathBuf::from(value));
                }
                "--arg" => {
                    args.push(
                        cursor
                            .next()
                            .ok_or_else(|| "--arg requires a value".to_owned())?,
                    );
                }
                "-h" | "--help" => return Err(smoke_usage().to_owned()),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }

        Ok(Self {
            scenario,
            node_count,
            command,
            args,
            marlin_home,
            host_home,
        })
    }
}

fn parse_positive_usize(option: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{option} requires a positive unsigned integer"))?;
    if parsed == 0 {
        return Err(format!("{option} requires a positive unsigned integer"));
    }
    Ok(parsed)
}

fn run_runtime_smoke(options: SmokeRuntimeOptions) -> Result<SmokeRuntimeReceipt, String> {
    let scenario = options.scenario.receipt_scenario();
    if options.scenario == SmokeScenarioOption::StateHomeEnv {
        return run_state_home_env_smoke(options);
    }
    if options.scenario == SmokeScenarioOption::ModelRouteDryRun {
        return run_model_route_dry_run_smoke();
    }

    let (run_id, graph, catalog) = match options.scenario {
        SmokeScenarioOption::BuiltinAdapters => (
            BUILTIN_ADAPTER_RUN_ID.to_owned(),
            builtin_adapter_graph(),
            DebugExecutorCatalog::with_builtin_debug_executors(),
        ),
        SmokeScenarioOption::ProcessCommandFanout => (
            PROCESS_COMMAND_RUN_ID.to_owned(),
            process_command_fanout_graph(options.node_count),
            DebugExecutorCatalog::new().register_process_command_tool(
                PROCESS_COMMAND_EXECUTOR,
                ProcessCommandBinding::new(
                    options.command.display().to_string(),
                    options.args,
                    None,
                    BTreeMap::new(),
                ),
            ),
        ),
        SmokeScenarioOption::ModelRouteDryRun => unreachable!("model-route-dry-run handled above"),
        SmokeScenarioOption::StateHomeEnv => unreachable!("state-home-env handled above"),
    };

    let graph_id = graph.graph_id.clone();
    let result = block_on(run_smoke_graph(
        GraphLoopExecutionRequest::new(run_id.clone(), graph.clone()),
        catalog,
    ))?;
    Ok(runtime_smoke_receipt(
        scenario, run_id, graph_id, graph, result,
    ))
}

fn run_state_home_env_smoke(options: SmokeRuntimeOptions) -> Result<SmokeRuntimeReceipt, String> {
    if options.marlin_home.is_none() && options.host_home.is_none() {
        return Err("state-home-env scenario requires --marlin-home or --host-home".to_owned());
    }

    let mut env = Vec::new();
    if let Some(path) = options.host_home {
        env.push(("HOME".to_owned(), path.display().to_string()));
    }
    if let Some(path) = options.marlin_home {
        env.push(("MARLIN_HOME".to_owned(), path.display().to_string()));
    }
    let request = RuntimeEnvironmentRequest::default().with_home_from_host_env(env);
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(request);
    let layout = resolution
        .environment
        .state_layout
        .ok_or_else(|| "state-home-env smoke did not resolve a runtime state layout".to_owned())?;

    let session_path = layout
        .session_path(STATE_HOME_ENV_RUN_ID)
        .ok_or_else(|| "state-home-env smoke could not resolve session path".to_owned())?
        .path;
    let memory_shard_path = layout
        .memory_shard_path(STATE_HOME_ENV_RUN_ID)
        .ok_or_else(|| "state-home-env smoke could not resolve memory shard path".to_owned())?
        .path;
    let receipt_path = layout
        .receipt_path(STATE_HOME_ENV_RUN_ID)
        .ok_or_else(|| "state-home-env smoke could not resolve receipt path".to_owned())?
        .path;
    let graph_cache_path = layout
        .graph_cache_path(STATE_HOME_ENV_RUN_ID)
        .ok_or_else(|| "state-home-env smoke could not resolve graph cache path".to_owned())?
        .path;
    let state_home = SmokeRuntimeStateHome {
        home: layout.home.path,
        source: layout.home.source,
        directory_count: layout.directories.len(),
        session_path,
        memory_shard_path,
        receipt_path,
        graph_cache_path,
    };

    Ok(SmokeRuntimeReceipt {
        scenario: SmokeRuntimeScenario::StateHomeEnv,
        llm_mode: SmokeLlmMode::NoLiveLlm,
        run_id: STATE_HOME_ENV_RUN_ID.to_owned(),
        graph_id: "state-home-env".to_owned(),
        terminal_status: GraphLoopExecutionStatus::Completed,
        passed: true,
        node_count: 0,
        visited_nodes: Vec::new(),
        node_receipt_count: 0,
        completed_node_receipt_count: 0,
        failed_node_receipt_count: 0,
        tool_spawn_count: 0,
        provider_spawn_count: 0,
        subagent_spawn_count: 0,
        process_spawn_count: 0,
        state_home: Some(state_home),
        model_route: None,
        diagnostics: Vec::new(),
        execution_result: None,
    })
}

fn run_model_route_dry_run_smoke() -> Result<SmokeRuntimeReceipt, String> {
    let rule = ModelRouteRule::new(
        "smoke-root-chat",
        100,
        ModelCommandMatcher::new()
            .with_command_kind_glob("chat")
            .with_agent_scope_glob("RootAgent")
            .with_workspace_glob("smoke://project/marlin-agent-core"),
        ModelEndpoint::new("openai", "gpt-5-mini"),
    )
    .with_session(
        ModelSessionPolicy::persistent(
            "smoke-root-chat-session",
            ModelContextForkMode::ForkSnapshot,
        )
        .with_requested_session_id("smoke-route-session"),
    );
    rule.validate_endpoint_contract()
        .map_err(|error| format!("model-route-dry-run endpoint contract failed: {error}"))?;
    let resolver = CompiledModelRouteResolver::new(vec![rule])
        .map_err(|error| format!("model-route-dry-run route compile failed: {error}"))?;
    let request = ModelRouteAdmissionRequest::chat(
        ModelRouteRequest::command(["marlin", "chat", "--dry-run"])
            .with_command_kind("chat")
            .with_agent_scope(ModelRouteAgentScope::RootAgent)
            .with_workspace("smoke://project/marlin-agent-core")
            .with_cwd("/workspace/marlin-agent-core"),
    )
    .with_latency_budget_ms(30_000)
    .with_evidence_profile("smoke-model-route-dry-run")
    .with_artifact_ref("marlin://smoke/model-route-dry-run");
    let response = admit_model_route_with_resolver(&resolver, request.clone())
        .map_err(|error| format!("model-route-dry-run admission failed: {error}"))?;
    let model_route = SmokeRuntimeModelRouteDryRun {
        rule_count: 1,
        request,
        response,
    };

    Ok(SmokeRuntimeReceipt {
        scenario: SmokeRuntimeScenario::ModelRouteDryRun,
        llm_mode: SmokeLlmMode::NoLiveLlm,
        run_id: MODEL_ROUTE_DRY_RUN_ID.to_owned(),
        graph_id: "model-route-dry-run".to_owned(),
        terminal_status: GraphLoopExecutionStatus::Completed,
        passed: true,
        node_count: 0,
        visited_nodes: Vec::new(),
        node_receipt_count: 0,
        completed_node_receipt_count: 0,
        failed_node_receipt_count: 0,
        tool_spawn_count: 0,
        provider_spawn_count: 0,
        subagent_spawn_count: 0,
        process_spawn_count: 0,
        state_home: None,
        model_route: Some(model_route),
        diagnostics: Vec::new(),
        execution_result: None,
    })
}

fn builtin_adapter_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "smoke-builtin-adapters".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "tool".to_owned(),
                executor: "debug.echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "provider".to_owned(),
                executor: "debug.provider.echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "subagent".to_owned(),
                executor: "debug.subagent.echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: Vec::new(),
    }
}

fn process_command_fanout_graph(node_count: usize) -> LoopGraph {
    let nodes = (0..node_count)
        .map(|index| LoopNodeSpec {
            id: format!("process-{index}"),
            executor: PROCESS_COMMAND_EXECUTOR.to_owned(),
            config: Default::default(),
        })
        .collect::<Vec<_>>();
    LoopGraph {
        graph_id: "smoke-process-command-fanout".to_owned(),
        nodes,
        edges: Vec::new(),
    }
}

async fn run_smoke_graph(
    request: GraphLoopExecutionRequest,
    catalog: DebugExecutorCatalog,
) -> Result<GraphLoopExecutionResult, String> {
    let (runtime, _events) = TokioAgentRuntime::new(64);
    let kernel = debug_kernel(&request.run_id, &request.graph, catalog)?;
    kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .map_err(|error| format!("runtime smoke graph task failed to join: {error}"))
}

fn runtime_smoke_receipt(
    scenario: SmokeRuntimeScenario,
    run_id: String,
    graph_id: String,
    graph: LoopGraph,
    result: GraphLoopExecutionResult,
) -> SmokeRuntimeReceipt {
    let diagnostics = result
        .diagnostics
        .iter()
        .cloned()
        .chain(
            result
                .node_receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter().cloned()),
        )
        .collect::<Vec<_>>();
    let completed_node_receipt_count = result
        .node_receipts
        .iter()
        .filter(|receipt| receipt.status == crate::GraphNodeExecutionStatus::Completed)
        .count();
    let failed_node_receipt_count = result.node_receipts.len() - completed_node_receipt_count;
    let tool_spawn_count = graph
        .nodes
        .iter()
        .filter(|node| node.executor == "debug.echo" || node.executor == PROCESS_COMMAND_EXECUTOR)
        .count();
    let provider_spawn_count = graph
        .nodes
        .iter()
        .filter(|node| node.executor == "debug.provider.echo")
        .count();
    let subagent_spawn_count = graph
        .nodes
        .iter()
        .filter(|node| node.executor == "debug.subagent.echo")
        .count();
    let process_spawn_count = graph
        .nodes
        .iter()
        .filter(|node| node.executor == PROCESS_COMMAND_EXECUTOR)
        .count();
    let terminal_status = result.status.clone();

    SmokeRuntimeReceipt {
        scenario,
        llm_mode: SmokeLlmMode::NoLiveLlm,
        run_id,
        graph_id,
        terminal_status,
        passed: result.status == GraphLoopExecutionStatus::Completed,
        node_count: graph.nodes.len(),
        visited_nodes: result.visited_nodes.clone(),
        node_receipt_count: result.node_receipts.len(),
        completed_node_receipt_count,
        failed_node_receipt_count,
        tool_spawn_count,
        provider_spawn_count,
        subagent_spawn_count,
        process_spawn_count,
        state_home: None,
        model_route: None,
        diagnostics,
        execution_result: Some(result),
    }
}
