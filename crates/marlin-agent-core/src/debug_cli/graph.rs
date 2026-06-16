//! `marlin graph ...` command implementations.

use std::{collections::BTreeSet, path::Path};

use crate::{
    GerbilLoopGraphPolicyCompilationRequest, GraphId, GraphLoopEvent, GraphLoopEventEnvelope,
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopIterationReport, GraphLoopKernel,
    GraphLoopStrategy, GraphLoopStrategyRuntime, GraphPolicyProposal, LoopGraph, RunId,
    TokioAgentRuntime, compile_gerbil_loop_graph_policy,
    protocol::{GraphQueryContext, GraphQueryFamily, GraphQueryRequest, GraphQueryResponse},
};
use marlin_org_memory::{
    MemoryOrgWorkspace, ProjectMemoryGraphStoreQuery, ToolCapabilityGraphStoreQuery,
    TopologyGraphStoreQuery,
};
use marlin_org_store::{FileSystemOrgSourceStore, OrgProjectRootCandidate, OrgSourceStore};
use marlin_org_workspace::OrgDocument;

use super::{
    MarlinCliResult,
    args::{ArgCursor, CommonOptions, GraphProposeOptions, GraphQueryOptions, GraphRunOptions},
    catalog::DebugExecutorCatalog,
    executor::{debug_kernel, read_debug_executor_catalog},
    graph_usage,
    io::{block_on, read_input, read_json_input},
    receipts::{
        GraphQueryOutput, GraphQuerySummary, LoopEventQuerySummary, LoopQuerySummary,
        LoopRunReceipt, ProjectRuntimeQuerySummary,
    },
};

pub(super) fn dispatch_graph(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing graph command\n{}", graph_usage()));
    };

    match command.as_str() {
        "query" => {
            let options = GraphQueryOptions::parse(cursor)?;
            let summary = run_graph_query(options)?;
            Ok(MarlinCliResult::success_json(&summary))
        }
        "propose" => {
            let options = GraphProposeOptions::parse(cursor)?;
            let proposal = propose_graph_policy(options)?;
            Ok(MarlinCliResult::success_json(&proposal))
        }
        "validate" => {
            let options = CommonOptions::parse(cursor)?;
            let proposal: GraphPolicyProposal = read_json_input(options.input.as_deref())?;
            let report = proposal.validate();
            Ok(MarlinCliResult::success_json(&report))
        }
        "run" => {
            let options = GraphRunOptions::parse(cursor)?;
            let request = read_graph_execution_request(options.input.as_deref(), &options.run_id)?;
            let catalog = read_debug_executor_catalog(options.catalog.as_deref())?;
            let result = block_on(run_graph_execution(request, catalog))?;
            Ok(MarlinCliResult::success_json(&result))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(graph_usage())),
        unknown => Err(format!(
            "unknown graph command `{unknown}`\n{}",
            graph_usage()
        )),
    }
}

fn run_graph_query(options: GraphQueryOptions) -> Result<GraphQueryOutput, String> {
    if options.family.is_some()
        && (!options.org_memory_fixtures.is_empty()
            || !options.org_memory_roots.is_empty()
            || options.org_memory_store_root.is_some()
            || !options.org_tool_roots.is_empty()
            || options.org_tool_store_root.is_some()
            || !options.org_topology_roots.is_empty()
            || options.org_topology_store_root.is_some())
    {
        return Err(
            "--family cannot be combined with Org-backed graph query options; set family in GraphQueryRequest JSON"
                .to_string(),
        );
    }

    if !options.org_topology_roots.is_empty() {
        if !options.org_memory_roots.is_empty() {
            return Err(
                "--org-topology-root cannot be combined with --org-memory-root".to_string(),
            );
        }
        if !options.org_tool_roots.is_empty() {
            return Err("--org-topology-root cannot be combined with --org-tool-root".to_string());
        }
        if !options.org_memory_fixtures.is_empty() {
            return Err(
                "--org-topology-root cannot be combined with --org-memory-fixture".to_string(),
            );
        }
        if options.org_memory_store_root.is_some() {
            return Err(
                "--org-memory-store-root cannot be combined with --org-topology-root".to_string(),
            );
        }
        if options.org_tool_store_root.is_some() {
            return Err(
                "--org-tool-store-root cannot be combined with --org-topology-root".to_string(),
            );
        }

        let request: GraphQueryRequest = read_json_input(options.input.as_deref())?;
        let workspace = MemoryOrgWorkspace::new();
        let store_root = options
            .org_topology_store_root
            .unwrap_or_else(|| ".".into());
        let store = FileSystemOrgSourceStore::new(store_root);
        ensure_store_roots_exist(&store, &options.org_topology_roots, "--org-topology-root")?;
        let candidates = options
            .org_topology_roots
            .into_iter()
            .map(OrgProjectRootCandidate::topology)
            .collect::<Vec<_>>();
        let response = workspace
            .query_topology_graph_from_store(TopologyGraphStoreQuery {
                receipt_id: options.receipt_id,
                request,
                store: &store,
                candidates,
            })
            .map_err(|error| format!("failed to query topology graph: {error}"))?;
        return Ok(GraphQueryOutput::ProjectRuntime(
            project_runtime_query_summary_from_response(response),
        ));
    }

    if options.org_topology_store_root.is_some() {
        return Err("--org-topology-store-root requires --org-topology-root".to_string());
    }

    if !options.org_tool_roots.is_empty() {
        if !options.org_memory_roots.is_empty() {
            return Err("--org-tool-root cannot be combined with --org-memory-root".to_string());
        }
        if !options.org_memory_fixtures.is_empty() {
            return Err("--org-tool-root cannot be combined with --org-memory-fixture".to_string());
        }
        if options.org_memory_store_root.is_some() {
            return Err(
                "--org-memory-store-root cannot be combined with --org-tool-root".to_string(),
            );
        }

        let request: GraphQueryRequest = read_json_input(options.input.as_deref())?;
        let workspace = MemoryOrgWorkspace::new();
        let store_root = options.org_tool_store_root.unwrap_or_else(|| ".".into());
        let store = FileSystemOrgSourceStore::new(store_root);
        ensure_store_roots_exist(&store, &options.org_tool_roots, "--org-tool-root")?;
        let candidates = options
            .org_tool_roots
            .into_iter()
            .map(OrgProjectRootCandidate::tool_capability)
            .collect::<Vec<_>>();
        let response = workspace
            .query_tool_capability_graph_from_store(ToolCapabilityGraphStoreQuery {
                receipt_id: options.receipt_id,
                request,
                store: &store,
                candidates,
            })
            .map_err(|error| format!("failed to query tool capability graph: {error}"))?;
        return Ok(GraphQueryOutput::ProjectRuntime(
            project_runtime_query_summary_from_response(response),
        ));
    }

    if options.org_tool_store_root.is_some() {
        return Err("--org-tool-store-root requires --org-tool-root".to_string());
    }

    if !options.org_memory_roots.is_empty() {
        if !options.org_memory_fixtures.is_empty() {
            return Err(
                "--org-memory-root cannot be combined with --org-memory-fixture".to_string(),
            );
        }

        let request: GraphQueryRequest = read_json_input(options.input.as_deref())?;
        let workspace = MemoryOrgWorkspace::new();
        let store_root = options.org_memory_store_root.unwrap_or_else(|| ".".into());
        let store = FileSystemOrgSourceStore::new(store_root);
        ensure_store_roots_exist(&store, &options.org_memory_roots, "--org-memory-root")?;
        let candidates = options
            .org_memory_roots
            .into_iter()
            .map(OrgProjectRootCandidate::project_memory)
            .collect::<Vec<_>>();
        let response = workspace
            .query_project_memory_graph_from_store(ProjectMemoryGraphStoreQuery {
                receipt_id: options.receipt_id,
                request,
                store: &store,
                candidates,
            })
            .map_err(|error| format!("failed to query project memory graph: {error}"))?;
        return Ok(GraphQueryOutput::ProjectRuntime(
            project_runtime_query_summary_from_response(response),
        ));
    }

    if options.org_memory_store_root.is_some() {
        return Err("--org-memory-store-root requires --org-memory-root".to_string());
    }

    if options.org_memory_fixtures.is_empty() {
        let input = read_input(options.input.as_deref())?;
        if let Some(family) = options.family {
            return graph_query_family_projection_output(&input, options.receipt_id, family);
        }
        return graph_query_output(&input);
    }

    let request: GraphQueryRequest = read_json_input(options.input.as_deref())?;
    let workspace = MemoryOrgWorkspace::new();
    for org_memory in &options.org_memory_fixtures {
        let memory_body = read_input(Some(org_memory.as_path()))?;
        let document_id = org_memory.display().to_string();
        workspace
            .load_document(OrgDocument::new(document_id, memory_body))
            .map_err(|error| {
                format!(
                    "failed to load Org memory document {}: {error}",
                    org_memory.display()
                )
            })?;
    }
    let response = query_loaded_org_graph(&workspace, options.receipt_id, request)?;
    Ok(GraphQueryOutput::ProjectRuntime(
        project_runtime_query_summary_from_response(response),
    ))
}

fn ensure_store_roots_exist<S: OrgSourceStore>(
    store: &S,
    roots: &[String],
    option_name: &str,
) -> Result<(), String> {
    let missing = roots
        .iter()
        .filter(|root| store.read_document(root).is_none())
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{option_name} points at missing Org root(s): {}",
            missing.join(", ")
        ))
    }
}

fn query_loaded_org_graph(
    workspace: &MemoryOrgWorkspace,
    receipt_id: impl Into<String>,
    request: GraphQueryRequest,
) -> Result<GraphQueryResponse, String> {
    match request.family.clone() {
        GraphQueryFamily::Memory => workspace
            .query_project_memory_graph(receipt_id, request)
            .map_err(|error| format!("failed to query project memory graph: {error}")),
        GraphQueryFamily::Tool => workspace
            .query_tool_capability_graph(receipt_id, request)
            .map_err(|error| format!("failed to query tool capability graph: {error}")),
        GraphQueryFamily::Session => workspace
            .query_session_graph(receipt_id, request)
            .map_err(|error| format!("failed to query session graph: {error}")),
        GraphQueryFamily::Content => workspace
            .query_content_graph(receipt_id, request)
            .map_err(|error| format!("failed to query content graph: {error}")),
        GraphQueryFamily::Topology => workspace
            .query_topology_graph(receipt_id, request)
            .map_err(|error| format!("failed to query topology graph: {error}")),
        GraphQueryFamily::Org => Err(
            "--org-memory-fixture does not execute GraphQueryFamily::Org; use a concrete read family"
                .to_string(),
        ),
        GraphQueryFamily::Evidence | GraphQueryFamily::Failure => Err(format!(
            "--org-memory-fixture does not execute GraphQueryFamily::{:?}; use a typed evidence/failure response receipt",
            request.family
        )),
    }
}

fn graph_query_summary(input: &str) -> Result<GraphQuerySummary, String> {
    let graph = parse_graph_input(input)?;
    let executors = graph
        .nodes
        .iter()
        .map(|node| node.executor.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let targeted = graph
        .edges
        .iter()
        .map(|edge| edge.to.as_str())
        .collect::<BTreeSet<_>>();
    let root_nodes = graph
        .nodes
        .iter()
        .filter(|node| !targeted.contains(node.id.as_str()))
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    Ok(GraphQuerySummary {
        graph_id: graph.graph_id,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        executors,
        root_nodes,
    })
}

fn graph_query_output(input: &str) -> Result<GraphQueryOutput, String> {
    match graph_query_summary(input) {
        Ok(summary) => Ok(GraphQueryOutput::Graph(summary)),
        Err(graph_error) => loop_query_summary(input)
            .map(GraphQueryOutput::Loop)
            .or_else(|loop_error| {
                loop_event_query_summary(input)
                    .map(GraphQueryOutput::LoopEvents)
                    .or_else(|event_error| {
                        project_runtime_query_summary(input)
                            .map(GraphQueryOutput::ProjectRuntime)
                            .map_err(|project_error| {
                                format!(
                                    "expected graph/proposal JSON, loop report JSON, graph-loop event JSON, or project-runtime graph query JSON: graph parse failed: {graph_error}; loop parse failed: {loop_error}; event parse failed: {event_error}; project-runtime parse failed: {project_error}"
                                )
                            })
                    })
            }),
    }
}

fn graph_query_family_projection_output(
    input: &str,
    receipt_id: impl Into<String>,
    family: GraphQueryFamily,
) -> Result<GraphQueryOutput, String> {
    let receipt_id = receipt_id.into();
    let context = GraphQueryContext::new("debug-loop-report");
    let response = match family {
        GraphQueryFamily::Evidence => GraphQueryResponse::from_iteration_reports_evidence(
            receipt_id,
            context,
            "loop report evidence",
            parse_loop_report_input(input)?,
        ),
        GraphQueryFamily::Failure => GraphQueryResponse::from_iteration_reports_failure(
            receipt_id,
            context,
            "loop report failures",
            parse_loop_report_input(input)?,
        ),
        unsupported => {
            return Err(format!(
                "--family {:?} is not supported for raw graph query input; use evidence or failure for loop-report projection, or pass a typed GraphQueryResponse JSON",
                unsupported
            ));
        }
    };
    Ok(GraphQueryOutput::ProjectRuntime(
        project_runtime_query_summary_from_response(response),
    ))
}

fn loop_query_summary(input: &str) -> Result<LoopQuerySummary, String> {
    let reports = parse_loop_report_input(input)?;
    let terminal = reports.last();
    Ok(LoopQuerySummary {
        run_ids: reports
            .iter()
            .map(|report| RunId::new(report.execution_result.snapshot.run_id.clone()))
            .collect(),
        graph_ids: reports
            .iter()
            .map(|report| GraphId::new(report.execution_result.snapshot.graph_id.clone()))
            .collect(),
        iteration_count: reports.len(),
        terminal_status: terminal.map(|report| report.execution_result.status.clone()),
        replayable: reports.iter().all(|report| report.trace.is_some()),
        missing_trace_count: reports
            .iter()
            .filter(|report| report.trace.is_none())
            .count(),
        statuses: reports
            .iter()
            .map(|report| report.execution_result.status.clone())
            .collect(),
        visited_nodes_by_iteration: reports
            .iter()
            .map(|report| report.execution_result.visited_nodes.clone())
            .collect(),
        diagnostic_count: reports
            .iter()
            .map(|report| report.execution_result.diagnostics.len())
            .sum(),
        node_receipt_count: reports
            .iter()
            .map(|report| report.execution_result.node_receipts.len())
            .sum(),
        continuation_receipt_count: reports
            .iter()
            .filter(|report| report.continuation_receipt.is_some())
            .count(),
        human_gate_receipt_count: reports
            .iter()
            .filter(|report| report.human_gate_receipt.is_some())
            .count(),
        human_decision_receipt_count: reports
            .iter()
            .filter(|report| report.human_decision_receipt.is_some())
            .count(),
        failure_classification_receipt_count: reports
            .iter()
            .filter(|report| report.failure_classification_receipt.is_some())
            .count(),
        failure_kinds: reports
            .iter()
            .filter_map(|report| report.failure_classification_receipt.as_ref())
            .map(|receipt| receipt.failure_kind.clone())
            .fold(Vec::new(), |mut kinds, kind| {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
                kinds
            }),
        trace_event_count: reports
            .iter()
            .filter_map(|report| report.trace.as_ref())
            .map(|trace| trace.events.len())
            .sum(),
    })
}

fn loop_event_query_summary(input: &str) -> Result<LoopEventQuerySummary, String> {
    let events = parse_loop_event_input(input)?;
    let terminal_status = events.iter().rev().find_map(|event| match &event.event {
        GraphLoopEvent::AgentEnd { status } => Some(status.clone()),
        _ => None,
    });
    let run_ids = events
        .iter()
        .map(|event| event.run_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let iteration_ids = events
        .iter()
        .filter_map(|event| event.iteration_id.map(|iteration_id| iteration_id.get()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let node_ids = events
        .iter()
        .filter_map(|event| {
            event
                .node_id
                .as_ref()
                .map(|node_id| node_id.as_str().to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let trace_ids = events
        .iter()
        .filter_map(|event| event.trace_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let event_types = events
        .iter()
        .map(|event| graph_loop_event_type(&event.event).to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let tool_event_count = events
        .iter()
        .filter(|event| graph_loop_event_is_tool_event(&event.event))
        .count();
    Ok(LoopEventQuerySummary {
        run_ids,
        event_count: events.len(),
        iteration_ids,
        node_ids,
        trace_ids,
        event_types,
        tool_event_count,
        terminal_status,
    })
}

fn project_runtime_query_summary(input: &str) -> Result<ProjectRuntimeQuerySummary, String> {
    let response = serde_json::from_str::<GraphQueryResponse>(input)
        .map_err(|error| format!("expected GraphQueryResponse JSON: {error}"))?;
    Ok(project_runtime_query_summary_from_response(response))
}

fn project_runtime_query_summary_from_response(
    response: GraphQueryResponse,
) -> ProjectRuntimeQuerySummary {
    ProjectRuntimeQuerySummary {
        receipt_id: response.receipt_id,
        family: response.request.family,
        query: response.request.query,
        match_count: response.matches.len(),
        source_project_ids: response
            .matches
            .iter()
            .map(|query_match| query_match.source_project_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        source_root_session_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.source_root_session_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        source_session_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.source_session_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        source_agent_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.source_agent_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        source_anchor_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.source_anchor_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        memory_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.memory_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        content_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.content_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        tool_capability_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.tool_capability_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        evidence_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.evidence_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        match_receipt_ids: response
            .matches
            .iter()
            .filter_map(|query_match| query_match.receipt_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        relationship_facts: response
            .matches
            .iter()
            .flat_map(|query_match| query_match.relationship.facts.iter().copied())
            .fold(Vec::new(), |mut facts, fact| {
                if !facts.contains(&fact) {
                    facts.push(fact);
                }
                facts
            }),
        score_basis_points: response
            .matches
            .iter()
            .map(|query_match| query_match.score_basis_points.as_u16())
            .collect(),
    }
}

fn parse_loop_event_input(input: &str) -> Result<Vec<GraphLoopEventEnvelope>, String> {
    serde_json::from_str::<Vec<GraphLoopEventEnvelope>>(input)
        .or_else(|_| serde_json::from_str::<GraphLoopEventEnvelope>(input).map(|event| vec![event]))
        .map_err(|error| {
            format!("expected GraphLoopEventEnvelope or GraphLoopEventEnvelope array JSON: {error}")
        })
}

fn graph_loop_event_type(event: &GraphLoopEvent) -> &'static str {
    match event {
        GraphLoopEvent::AgentStart { .. } => "agent_start",
        GraphLoopEvent::TurnStart => "turn_start",
        GraphLoopEvent::MessageStart { .. } => "message_start",
        GraphLoopEvent::MessageUpdate { .. } => "message_update",
        GraphLoopEvent::MessageEnd { .. } => "message_end",
        GraphLoopEvent::ToolExecutionStart { .. } => "tool_execution_start",
        GraphLoopEvent::ToolExecutionUpdate { .. } => "tool_execution_update",
        GraphLoopEvent::ToolExecutionEnd { .. } => "tool_execution_end",
        GraphLoopEvent::TurnEnd { .. } => "turn_end",
        GraphLoopEvent::AgentEnd { .. } => "agent_end",
    }
}

fn graph_loop_event_is_tool_event(event: &GraphLoopEvent) -> bool {
    matches!(
        event,
        GraphLoopEvent::ToolExecutionStart { .. }
            | GraphLoopEvent::ToolExecutionUpdate { .. }
            | GraphLoopEvent::ToolExecutionEnd { .. }
    )
}

fn parse_loop_report_input(input: &str) -> Result<Vec<GraphLoopIterationReport>, String> {
    serde_json::from_str::<Vec<GraphLoopIterationReport>>(input)
        .or_else(|_| {
            serde_json::from_str::<GraphLoopIterationReport>(input).map(|report| vec![report])
        })
        .or_else(|_| serde_json::from_str::<LoopRunReceipt>(input).map(|receipt| receipt.reports))
        .map_err(|error| {
            format!(
                "expected GraphLoopIterationReport, GraphLoopIterationReport array, or LoopRunReceipt JSON: {error}"
            )
        })
}

fn propose_graph_policy(options: GraphProposeOptions) -> Result<GraphPolicyProposal, String> {
    match options.strategy.as_str() {
        "static" => {
            let graph: LoopGraph = read_json_input(options.input.as_deref())?;
            Ok(GraphPolicyProposal::new(
                GraphLoopStrategy::new(
                    options.strategy_id,
                    options.version,
                    GraphLoopStrategyRuntime::StaticPolicy,
                ),
                graph,
                options.input_digest,
                options.output_digest,
            ))
        }
        "gerbil" => {
            let request: GerbilLoopGraphPolicyCompilationRequest =
                read_json_input(options.input.as_deref())?;
            compile_gerbil_loop_graph_policy(request)
                .map_err(|error| format!("failed to compile Gerbil graph policy: {error:?}"))
        }
        unknown => Err(format!(
            "unsupported graph proposal strategy `{unknown}`; expected static or gerbil"
        )),
    }
}

fn read_graph_execution_request(
    input: Option<&Path>,
    default_run_id: &str,
) -> Result<GraphLoopExecutionRequest, String> {
    let raw = read_input(input)?;
    serde_json::from_str::<GraphLoopExecutionRequest>(&raw).or_else(|_| {
        parse_graph_input(&raw).map(|graph| GraphLoopExecutionRequest::new(default_run_id, graph))
    })
}

pub(super) fn parse_graph_input(input: &str) -> Result<LoopGraph, String> {
    serde_json::from_str::<LoopGraph>(input)
        .or_else(|_| {
            serde_json::from_str::<GraphPolicyProposal>(input)
                .map(|proposal| proposal.proposed_graph)
        })
        .map_err(|error| format!("expected LoopGraph or GraphPolicyProposal JSON: {error}"))
}

async fn run_graph_execution(
    request: GraphLoopExecutionRequest,
    catalog: DebugExecutorCatalog,
) -> Result<GraphLoopExecutionResult, String> {
    let (runtime, _events) = TokioAgentRuntime::new(64);
    let kernel = debug_kernel(&request.run_id, &request.graph, catalog)?;
    kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .map_err(|error| format!("graph execution task failed to join: {error}"))
}
