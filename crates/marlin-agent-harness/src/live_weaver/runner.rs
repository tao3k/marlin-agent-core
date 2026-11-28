// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Headless multi-turn live Weaver agent evaluation runner wired directly to:
//! - Declarative Scheme scenarios & Cedar governance (`HarnessSchemeBridge`)
//! - Fail-closed Cedar Governance Gate (`CompiledHarnessGovernanceGate`)
//! - `TraceRecorder` OTel-compatible span capture (`WithSubscriber`)
//! - Full Causal Trajectory DAG verified via Bi-Temporal Causality in Scheme
//! - `AgentHarnessEvidenceGraph` projection

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use marlin_agent_protocol::{
    AgentTraceSpanRecord, ModelGatewayCompletionOptions, ModelGatewayError, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelToolDefinition, tool_gateway_message,
    user_gateway_message,
};
use marlin_agent_runtime::{
    ModelToolCapability, ModelToolDeclaration, ModelToolOutcome, TokioAgentRuntime,
    TokioAgentRuntimeBuildRequest, ToolRuntime,
};
use marlin_agent_search_engine::{
    SearchDslSessionId, WorkspaceIndexConfig, default_workspace_edb_cache_path,
    default_workspace_snapshot_path, hydrate_workspace_open_edb_org_and_cache,
    open_or_build_workspace_index_safe,
};
use marlin_agent_stream::{
    ModelStreamEvent, ModelStreamingGateway, matches_observed_anthropic_thinking,
};
use marlin_agent_tooling::{
    GrpcSearchToolHandler, LoopbackSearchGrpcServer, SearchGatewayService, SharedSearchIndexSlot,
    catalog_from_declarations_with_search, tool_schema_entries_from_declarations,
};
use tokio_stream::StreamExt;
use tracing::Instrument;
use tracing::instrument::WithSubscriber;

use crate::{AgentHarnessEvidenceGraph, TraceRecorder};

use super::artifacts::{
    ScenarioEvidenceGraphInputs, TrajectoryBuilder, build_reflection_message,
    build_scenario_evidence_graph, parse_scenario_reflection,
};
use super::config::{LiveEvalBudget, LiveWeaverRoute};
use super::governance::{CompiledHarnessGovernanceGate, GovernanceDecisionReceipt};
use super::report::{
    DetectedGap, ParsedSearchReceipt, QueryTierKind, classify_tool_call, detect_scenario_gaps,
    parse_search_receipt, score_ground_truth,
};
use super::scheme_bridge::{
    HarnessSchemeBridge, ProjectedHarnessScenario, TemporalCausalityVerdict,
};

/// Semantic category of one causal trajectory node recorded during a live run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrajectoryEventKind {
    PromptSubmitted,
    ModelTurnCompleted,
    GovernancePermitted,
    GovernanceDenied,
    ToolSearchCompleted,
    ToolSearchFailed,
    ToolDenied,
    ToolWriteExecuted,
    FinalAnswerProduced,
}

impl TrajectoryEventKind {
    /// Canonical Scheme runtime symbol consumed by `analyze-harness-trajectory`.
    #[must_use]
    pub fn runtime_symbol(self) -> &'static str {
        match self {
            Self::PromptSubmitted => "prompt",
            Self::ModelTurnCompleted => "turn",
            Self::GovernancePermitted => "gov_permit",
            Self::GovernanceDenied => "gov_forbid",
            Self::ToolSearchCompleted => "tool_search_ok",
            Self::ToolSearchFailed => "tool_search_failed",
            Self::ToolDenied => "tool_denied",
            Self::ToolWriteExecuted => "tool_write_executed",
            Self::FinalAnswerProduced => "final_answer",
        }
    }
}

/// One node in the scenario's bi-temporal causal execution trajectory DAG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryEvent {
    pub event_id: String,
    pub kind: TrajectoryEventKind,
    pub logical_pos: usize,
    pub parent_ids: Vec<String>,
    pub valid_pos: usize,
    pub knowledge_pos: usize,
    pub detail: String,
}

impl TrajectoryEvent {
    #[must_use]
    pub fn runtime_symbol(&self) -> &'static str {
        self.kind.runtime_symbol()
    }
}

/// Per-turn model streaming & continuation verification telemetry.
#[derive(Clone, Debug)]
pub struct TurnTelemetry {
    pub round: usize,
    pub turn_ms: u64,
    pub thinking_bytes: usize,
    pub text_bytes: usize,
    pub tool_calls: usize,
    pub finish_reason: Option<String>,
    pub thinking_verified: bool,
}

/// Per-tool-call execution, latency, tier, governance, and receipt telemetry.
#[derive(Clone, Debug)]
pub struct ToolCallTelemetry {
    pub round: usize,
    pub call_id: String,
    pub tool_name: String,
    pub argument: String,
    pub tier: QueryTierKind,
    pub status: &'static str,
    pub elapsed_ms: u64,
    pub output_bytes: usize,
    pub receipt: ParsedSearchReceipt,
    pub governance: GovernanceDecisionReceipt,
    pub error_or_preview: String,
}

/// One structured post-run reflection entry produced by the model explaining
/// why a query failed, required AST repair, or returned 0 hits — explicitly
/// surfaced for subsequent human correctness audit.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ReflectionItem {
    #[serde(default)]
    pub round: usize,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub symptom: String,
    #[serde(default)]
    pub why_i_wrote_it_wrong: String,
    #[serde(default)]
    pub attribution: String,
    #[serde(default)]
    pub proposed_improvement: String,
}

/// Structured post-run self-reflection artifact captured in a tool-free turn
/// after scenario execution completes.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ScenarioReflection {
    #[serde(default)]
    pub overall_assessment: String,
    #[serde(default)]
    pub items: Vec<ReflectionItem>,
    #[serde(default)]
    pub top_dsl_design_recommendations: Vec<String>,
    #[serde(skip)]
    pub raw_response: String,
    #[serde(skip)]
    pub parsed_structured: bool,
    #[serde(skip)]
    pub reflection_ms: u64,
}

/// Complete typed evaluation outcome of one live scenario run, unifying
/// Governance, OTel Telemetry Spans, Causal Trajectory, Bi-Temporal Causality
/// Proof, EvidenceGraph, Ground-Truth Scoring, and optional Model Reflection.
#[derive(Clone, Debug)]
pub struct LiveScenarioReport {
    pub scenario_id: String,
    pub title: String,
    pub message: String,
    pub governance_policy_id: String,
    pub write_allowed: bool,
    pub target_max_rounds: usize,
    pub rounds_used: usize,
    pub wall_ms: u64,
    pub turns: Vec<TurnTelemetry>,
    pub tool_calls: Vec<ToolCallTelemetry>,
    pub governance_receipts: Vec<GovernanceDecisionReceipt>,
    pub trajectory: Vec<TrajectoryEvent>,
    pub temporal_causality: TemporalCausalityVerdict,
    pub telemetry_spans: Vec<AgentTraceSpanRecord>,
    pub evidence_graph: AgentHarnessEvidenceGraph,
    pub final_answer: String,
    pub matched_checkpoints: usize,
    pub total_checkpoints: usize,
    pub missed_checkpoints: Vec<String>,
    pub gaps: Vec<DetectedGap>,
    pub reflection: Option<ScenarioReflection>,
    pub aborted_reason: Option<String>,
}

/// Reusable headless live-Weaver evaluation session backed by Scheme-projected
/// policies, Cedar governance, and the workspace's resident index + Open EDB.
pub struct LiveWeaverEvalSession {
    scheme_bridge: HarnessSchemeBridge,
    route: LiveWeaverRoute,
    budget: LiveEvalBudget,
    tool_options: ModelGatewayCompletionOptions,
    offered_definitions: Vec<ModelToolDefinition>,
    tool_capabilities: BTreeMap<String, &'static str>,
    search_grpc_server: LoopbackSearchGrpcServer,
    active_search_session: Arc<RwLock<SearchDslSessionId>>,
    search_handlers: BTreeMap<String, Arc<GrpcSearchToolHandler>>,
    runtime_owner: TokioAgentRuntime,
    indexed_files: usize,
    hydrated_files: usize,
}

impl LiveWeaverEvalSession {
    /// Bootstrap a headless live evaluation session from a workspace root,
    /// Scheme policy bridge, and resolved Weaver route.
    pub fn open(
        repo_root: &Path,
        scheme_bridge: &HarnessSchemeBridge,
        route: LiveWeaverRoute,
        budget: LiveEvalBudget,
    ) -> Result<Self, String> {
        let declarations: Vec<ModelToolDeclaration> =
            scheme_bridge.project_tooling_declarations()?;
        let helm_policy = scheme_bridge.project_helm_index_policy()?;

        let tool_capabilities: BTreeMap<String, &'static str> = declarations
            .iter()
            .map(|decl| {
                let cap = match decl.capability() {
                    ModelToolCapability::Shell => "shell",
                    ModelToolCapability::Search => "search",
                };
                (decl.definition().name.as_str().to_owned(), cap)
            })
            .collect();

        let index_cfg = WorkspaceIndexConfig {
            skip_dirs: helm_policy.index.skip_dirs,
            max_line_matches_per_hit: 32,
            ..WorkspaceIndexConfig::default()
        };

        let (snapshot_path, edb_cache_path) = resolve_workspace_search_cache_paths(repo_root);
        let mut index = open_or_build_workspace_index_safe(repo_root, &snapshot_path, index_cfg);
        let hydrate_receipt = hydrate_workspace_open_edb_org_and_cache(&mut index, &edb_cache_path);
        let indexed_files = index.file_count();
        let hydrated_files = hydrate_receipt.restored_files + hydrate_receipt.org_files_hydrated;

        let shared_index = Arc::new(RwLock::new(index));
        let search_decls: Vec<ModelToolDeclaration> = declarations
            .iter()
            .filter(|decl| decl.capability() == ModelToolCapability::Search)
            .cloned()
            .collect();
        if search_decls.is_empty() {
            return Err(
                "Scheme ToolingCatalogValue does not declare any `search` capability tools"
                    .to_owned(),
            );
        }

        let (runtime_owner, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(16),
        );

        let grpc_service = SearchGatewayService::new(
            Arc::clone(&shared_index),
            tool_schema_entries_from_declarations(&search_decls),
            scheme_bridge.search_session_dsl_runner(),
            runtime_owner.context(),
        );
        let search_grpc_server = LoopbackSearchGrpcServer::spawn(grpc_service, &runtime_owner)?;
        let default_sid = SearchDslSessionId::new("harness-default")
            .map_err(|e| format!("invalid default search session id: {e}"))?;
        let active_search_session = Arc::new(RwLock::new(default_sid));
        let search_handlers: BTreeMap<String, Arc<GrpcSearchToolHandler>> = search_decls
            .iter()
            .map(|decl| {
                let name = decl.definition().name.as_str().to_owned();
                let handler = Arc::new(GrpcSearchToolHandler::new(
                    decl.clone(),
                    search_grpc_server.client(),
                    Arc::clone(&active_search_session),
                ));
                (name, handler)
            })
            .collect();

        let slot = SharedSearchIndexSlot::with_sync_debounce(
            Some(shared_index),
            Duration::from_secs(3600),
        );
        let catalog =
            catalog_from_declarations_with_search(declarations, repo_root.to_path_buf(), slot)
                .map_err(|err| format!("failed to build ModelToolCatalog: {err:?}"))?;
        let offer = catalog.offer();
        let offered_definitions = offer.definitions();
        let tool_options = offer.completion_options();

        Ok(Self {
            scheme_bridge: scheme_bridge.clone(),
            route,
            budget,
            tool_options,
            offered_definitions,
            tool_capabilities,
            search_grpc_server,
            active_search_session,
            search_handlers,
            runtime_owner,
            indexed_files,
            hydrated_files,
        })
    }

    #[must_use]
    pub fn route(&self) -> &LiveWeaverRoute {
        &self.route
    }

    #[must_use]
    pub fn offered_definitions(&self) -> &[ModelToolDefinition] {
        &self.offered_definitions
    }

    #[must_use]
    pub fn indexed_files(&self) -> usize {
        self.indexed_files
    }

    #[must_use]
    pub fn hydrated_files(&self) -> usize {
        self.hydrated_files
    }

    /// Execute one Scheme-projected scenario end-to-end against live Weaver
    /// under Cedar governance, `TraceRecorder` telemetry, and Bi-Temporal
    /// Causality trajectory verification.
    pub async fn run_scenario(
        &self,
        scenario: &ProjectedHarnessScenario,
    ) -> Result<LiveScenarioReport, String> {
        let governance_gate = CompiledHarnessGovernanceGate::compile(scenario.governance.clone())?;
        let recorder = TraceRecorder::new();
        let scenario_span = tracing::info_span!(
            "harness.scenario.run",
            scenario.id = scenario.id.as_str(),
            governance.policy_id = scenario.governance.policy_id.as_str(),
            governance.write_allowed = scenario.governance.write_allowed,
            model.provider = self.route.endpoint.provider.as_str(),
            model.id = self.route.endpoint.model.as_str(),
        );

        let report = self
            .run_scenario_traced(scenario, &governance_gate, &recorder)
            .instrument(scenario_span)
            .with_subscriber(recorder.clone())
            .await?;

        Ok(report)
    }

    async fn run_scenario_traced(
        &self,
        scenario: &ProjectedHarnessScenario,
        governance_gate: &CompiledHarnessGovernanceGate,
        recorder: &TraceRecorder,
    ) -> Result<LiveScenarioReport, String> {
        let started = Instant::now();
        let session_key = SearchDslSessionId::new(format!("scenario-{}", scenario.id))
            .map_err(|e| format!("invalid scenario search session id: {e}"))?;
        if let Ok(mut guard) = self.active_search_session.write() {
            *guard = session_key.clone();
        }
        let _ = self.search_grpc_server.reset_session(&session_key).await?;
        let max_rounds = scenario.max_rounds.min(self.budget.max_rounds).max(1);
        let max_tool_calls = scenario
            .max_tool_calls
            .min(self.budget.max_tool_calls)
            .max(1);

        // Send ONLY the user's natural-language message (zero pre-seeded coaching
        // system prompt): tool discovery and selection must emerge purely from
        // declared tools and structured tool receipts.
        let mut messages: Vec<ModelGatewayMessage> =
            vec![user_gateway_message(scenario.message.as_str())];

        let mut trajectory = TrajectoryBuilder::new(started);
        let prompt_ev = trajectory.push(
            TrajectoryEventKind::PromptSubmitted,
            Vec::new(),
            format!("scenario={}", scenario.id),
        );
        let mut last_frontier = vec![prompt_ev];

        let mut turns = Vec::new();
        let mut tool_calls = Vec::new();
        let mut governance_receipts = Vec::new();
        let mut final_answer = String::new();
        let mut aborted_reason = None;
        let mut total_calls = 0usize;

        let mut budget_hit_note: Option<String> = None;
        for round in 1..=max_rounds {
            // Reserve the final round (`round == max_rounds` when `max_rounds > 1`)
            // or the turn immediately after `max_tool_calls` is exhausted as a
            // tool-free synthesis turn so total rounds strictly stay `<= max_rounds`
            // (and never overflow into a 5th+ round).
            let is_synthesis_turn =
                (max_rounds > 1 && round == max_rounds && !tool_calls.is_empty())
                    || budget_hit_note.is_some();
            if is_synthesis_turn {
                messages.push(user_gateway_message(
                    "No further tool calls are available on this final turn. Do not emit any tool invocations or markup; write your complete, verified final answer directly from the evidence already gathered.",
                ));
            }
            let turn_started = Instant::now();
            let turn_span = tracing::info_span!(
                "harness.turn",
                scenario.id = scenario.id.as_str(),
                turn.round = round as u64,
                turn.synthesis_only = is_synthesis_turn,
            );
            let request = if is_synthesis_turn {
                ModelGatewayRequest::new(self.route.endpoint.clone(), messages.clone())
            } else {
                ModelGatewayRequest::new(self.route.endpoint.clone(), messages.clone())
                    .with_options(self.tool_options.clone())
            };

            let stream_res = collect_streaming_turn(self.route.gateway.as_ref(), request)
                .instrument(turn_span)
                .await;
            let turn_ms = u64::try_from(turn_started.elapsed().as_millis()).unwrap_or(u64::MAX);

            let (text, thinking, choice_opt, tool_requested) = match stream_res {
                Ok(v) => v,
                Err(err) => {
                    aborted_reason = Some(format!("round {round} gateway error: {err}"));
                    break;
                }
            };
            let Some(choice) = choice_opt else {
                aborted_reason = Some(format!("round {round} returned no assistant choice"));
                break;
            };

            let thinking_verified = thinking.is_empty()
                || matches_observed_anthropic_thinking(&choice.message, &thinking);
            turns.push(TurnTelemetry {
                round,
                turn_ms,
                thinking_bytes: thinking.len(),
                text_bytes: text.len().max(choice.message.content.len()),
                tool_calls: if is_synthesis_turn {
                    0
                } else {
                    choice.message.tool_calls.len()
                },
                finish_reason: choice.finish_reason.clone(),
                thinking_verified,
            });

            let turn_ev = trajectory.push(
                TrajectoryEventKind::ModelTurnCompleted,
                last_frontier.clone(),
                if is_synthesis_turn {
                    format!(
                        "round={round} synthesis_only=true finish={:?}",
                        choice.finish_reason
                    )
                } else {
                    format!(
                        "round={round} tools={} finish={:?}",
                        choice.message.tool_calls.len(),
                        choice.finish_reason
                    )
                },
            );

            let assistant_msg = choice.message;
            let requested_calls = assistant_msg.tool_calls.clone();
            if is_synthesis_turn || (!tool_requested && requested_calls.is_empty()) {
                final_answer = if assistant_msg.content.is_empty() {
                    text
                } else {
                    assistant_msg.content
                };
                if final_answer.trim().is_empty() {
                    aborted_reason = budget_hit_note.or_else(|| {
                        Some(format!("round {round} returned empty final answer"))
                    });
                } else {
                    let _final_ev = trajectory.push(
                        TrajectoryEventKind::FinalAnswerProduced,
                        vec![turn_ev],
                        format!("chars={}", final_answer.chars().count()),
                    );
                }
                break;
            }

            messages.push(assistant_msg);

            struct PlannedToolCall {
                call: marlin_agent_protocol::ModelToolCall,
                call_id_str: String,
                tool_name: String,
                argument: String,
                tier: QueryTierKind,
                gov_receipt: GovernanceDecisionReceipt,
                gov_ev: String,
            }

            let mut round_tool_events = Vec::with_capacity(requested_calls.len());
            let mut planned: Vec<PlannedToolCall> = Vec::with_capacity(requested_calls.len());
            for call in requested_calls {
                if budget_hit_note.is_some() {
                    let msg = "Tool call skipped: max_tool_calls budget reached; synthesize your final answer now.".to_owned();
                    let skip_ev = trajectory.push(
                        TrajectoryEventKind::ToolDenied,
                        vec![turn_ev.clone()],
                        format!("tool={} status=budget_skipped", call.name.as_str()),
                    );
                    round_tool_events.push(skip_ev);
                    messages.push(tool_gateway_message(call.id, msg));
                    continue;
                }
                total_calls += 1;
                let tool_name = call.name.as_str().to_owned();
                let call_id_str = call.id.as_str().to_owned();
                let argument = call
                    .arguments
                    .get("query")
                    .or_else(|| call.arguments.get("command"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        serde_json::to_string(&call.arguments).unwrap_or_else(|e| {
                            format!("<serialization-error:{e}> {:?}", call.arguments)
                        })
                    });
                let tier = classify_tool_call(&tool_name, &argument);
                let declared_cap = self.tool_capabilities.get(&tool_name).copied();

                let gov_receipt = governance_gate.evaluate_tool_call(
                    &scenario.id,
                    &call_id_str,
                    &tool_name,
                    declared_cap,
                );
                let gov_ev = trajectory.push(
                    if gov_receipt.permitted {
                        TrajectoryEventKind::GovernancePermitted
                    } else {
                        TrajectoryEventKind::GovernanceDenied
                    },
                    vec![turn_ev.clone()],
                    format!(
                        "tool={tool_name} cap={} is_write={} cedar={}",
                        gov_receipt.capability, gov_receipt.is_write, gov_receipt.cedar_decision
                    ),
                );
                governance_receipts.push(gov_receipt.clone());
                planned.push(PlannedToolCall {
                    call,
                    call_id_str,
                    tool_name,
                    argument,
                    tier,
                    gov_receipt,
                    gov_ev,
                });

                if total_calls >= max_tool_calls {
                    budget_hit_note = Some(format!("reached max_tool_calls budget ({max_tool_calls})"));
                }
            }

            struct ConcurrentToolOutcome {
                elapsed_ms: u64,
                status: &'static str,
                body: String,
                receipt: ParsedSearchReceipt,
                preview: String,
                exec_kind: TrajectoryEventKind,
            }

            // Execute all Cedar-permitted read-only search calls in this turn CONCURRENTLY
            // (preserving deterministic original call order when reassembling results/DAG).
            let mut join_set = tokio::task::JoinSet::new();
            for (idx, item) in planned.iter().enumerate() {
                if !item.gov_receipt.permitted {
                    let actionable_denial = super::governance::format_actionable_denial_message(
                        &item.gov_receipt.reason,
                        &item.tool_name,
                        &item.argument,
                    );
                    join_set.spawn(async move {
                        (
                            idx,
                            ConcurrentToolOutcome {
                                elapsed_ms: 0,
                                status: "denied",
                                body: actionable_denial.clone(),
                                receipt: ParsedSearchReceipt::default(),
                                preview: actionable_denial,
                                exec_kind: TrajectoryEventKind::ToolDenied,
                            },
                        )
                    });
                } else if let Some(handler) = self.search_handlers.get(item.tool_name.as_str()) {
                    let handler = Arc::clone(handler);
                    let call = item.call.clone();
                    let rt_ctx = self.runtime_owner.context();
                    let tool_name = item.tool_name.clone();
                    let call_id_str = item.call_id_str.clone();
                    let tier_str = item.tier.as_str();
                    let dispatch = tracing::dispatcher::get_default(|d| d.clone());
                    join_set.spawn(
                        async move {
                            let call_started = Instant::now();
                            let exec_span = tracing::info_span!(
                                "runtime.tool.execute",
                                tool.name = tool_name.as_str(),
                                tool.call_id = call_id_str.as_str(),
                                tool.tier = tier_str,
                            );
                            let outcome = handler
                                .run_tool(call, rt_ctx)
                                .instrument(exec_span)
                                .await;
                            let elapsed_ms =
                                u64::try_from(call_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                            let res = match outcome {
                                ModelToolOutcome::Completed(body) => {
                                    let receipt = parse_search_receipt(&body);
                                    let preview = body
                                        .lines()
                                        .filter(|line| {
                                            !line.starts_with("generation:")
                                                && !line.starts_with("elapsed_us:")
                                                && !line.starts_with("indexed_files:")
                                        })
                                        .take(14)
                                        .collect::<Vec<_>>()
                                        .join(" | ");
                                    ConcurrentToolOutcome {
                                        elapsed_ms,
                                        status: "completed",
                                        body,
                                        receipt,
                                        preview,
                                        exec_kind: TrajectoryEventKind::ToolSearchCompleted,
                                    }
                                }
                                ModelToolOutcome::Failed(err) => {
                                    let msg = format!("{tool_name} error: {err}");
                                    ConcurrentToolOutcome {
                                        elapsed_ms,
                                        status: "failed",
                                        body: msg,
                                        receipt: ParsedSearchReceipt::default(),
                                        preview: err,
                                        exec_kind: TrajectoryEventKind::ToolSearchFailed,
                                    }
                                }
                            };
                            (idx, res)
                        }
                        .with_subscriber(dispatch),
                    );
                } else {
                    let denied = format!(
                        "Fail-closed harness refused unimplemented tool `{}` even after policy check.",
                        item.tool_name
                    );
                    join_set.spawn(async move {
                        (
                            idx,
                            ConcurrentToolOutcome {
                                elapsed_ms: 0,
                                status: "denied",
                                body: denied.clone(),
                                receipt: ParsedSearchReceipt::default(),
                                preview: denied,
                                exec_kind: TrajectoryEventKind::ToolDenied,
                            },
                        )
                    });
                }
            }

            let mut completed_results: Vec<Option<ConcurrentToolOutcome>> =
                (0..planned.len()).map(|_| None).collect();
            while let Some(joined) = join_set.join_next().await {
                if let Ok((idx, outcome)) = joined {
                    completed_results[idx] = Some(outcome);
                }
            }

            for (item, outcome) in planned.into_iter().zip(completed_results) {
                let outcome = outcome.unwrap_or_else(|| {
                    let msg = format!("{} error: concurrent search task panicked", item.tool_name);
                    ConcurrentToolOutcome {
                        elapsed_ms: 0,
                        status: "failed",
                        body: msg.clone(),
                        receipt: ParsedSearchReceipt::default(),
                        preview: msg,
                        exec_kind: TrajectoryEventKind::ToolSearchFailed,
                    }
                });
                let tool_ev = trajectory.push(
                    outcome.exec_kind,
                    vec![item.gov_ev],
                    format!(
                        "tool={} status={} ms={}",
                        item.tool_name, outcome.status, outcome.elapsed_ms
                    ),
                );
                round_tool_events.push(tool_ev);

                let output_bytes = outcome.body.len();
                tool_calls.push(ToolCallTelemetry {
                    round,
                    call_id: item.call_id_str,
                    tool_name: item.tool_name,
                    argument: item.argument,
                    tier: item.tier,
                    status: outcome.status,
                    elapsed_ms: outcome.elapsed_ms,
                    output_bytes,
                    receipt: outcome.receipt,
                    governance: item.gov_receipt,
                    error_or_preview: outcome.preview,
                });
                messages.push(tool_gateway_message(item.call.id, outcome.body));
            }

            if !round_tool_events.is_empty() {
                last_frontier = round_tool_events;
            }
            if round == max_rounds && final_answer.is_empty() {
                aborted_reason = Some(format!("reached max_rounds budget ({max_rounds}) without final answer"));
            }
        }

        let trajectory_events = trajectory.into_events();
        let temporal_causality = self.scheme_bridge.analyze_trajectory(
            &scenario.id,
            scenario.governance.write_allowed,
            trajectory_events.clone(),
        )?;

        let rounds_used = turns.len();
        let (matched_checkpoints, missed_checkpoints) =
            score_ground_truth(&final_answer, &scenario.checkpoints);
        let gaps = detect_scenario_gaps(
            scenario,
            rounds_used,
            &turns,
            &tool_calls,
            &temporal_causality,
            &missed_checkpoints,
            aborted_reason.as_deref(),
        );
        let reflection = if (scenario.enable_reflection || self.budget.force_reflection)
            && aborted_reason.is_none()
        {
            Some(
                self.run_reflection_turn(scenario, rounds_used, &tool_calls, &gaps)
                    .await,
            )
        } else {
            None
        };

        let telemetry_spans = recorder.spans();
        let evidence_graph = build_scenario_evidence_graph(&ScenarioEvidenceGraphInputs {
            scenario,
            governance_receipts: &governance_receipts,
            tool_calls: &tool_calls,
            temporal: &temporal_causality,
            spans: &telemetry_spans,
            gaps: &gaps,
            matched_checkpoints,
            total_checkpoints: scenario.checkpoints.len(),
        });

        Ok(LiveScenarioReport {
            scenario_id: scenario.id.clone(),
            title: scenario.title.clone(),
            message: scenario.message.clone(),
            governance_policy_id: scenario.governance.policy_id.clone(),
            write_allowed: scenario.governance.write_allowed,
            target_max_rounds: scenario.target_max_rounds,
            rounds_used,
            wall_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            turns,
            tool_calls,
            governance_receipts,
            trajectory: trajectory_events,
            temporal_causality,
            telemetry_spans,
            evidence_graph,
            final_answer,
            matched_checkpoints,
            total_checkpoints: scenario.checkpoints.len(),
            missed_checkpoints,
            gaps,
            reflection,
            aborted_reason,
        })
    }

    async fn run_reflection_turn(
        &self,
        scenario: &ProjectedHarnessScenario,
        rounds_used: usize,
        tool_calls: &[ToolCallTelemetry],
        gaps: &[DetectedGap],
    ) -> ScenarioReflection {
        let started = Instant::now();
        let span = tracing::info_span!(
            "harness.reflection",
            scenario.id = scenario.id.as_str(),
            tool_calls = tool_calls.len() as u64,
        );

        let reflection_message = build_reflection_message(
            scenario,
            &self.offered_definitions,
            rounds_used,
            tool_calls,
            gaps,
        );
        let messages = vec![user_gateway_message(reflection_message)];
        let request = ModelGatewayRequest::new(self.route.endpoint.clone(), messages);
        let outcome = collect_streaming_turn(self.route.gateway.as_ref(), request)
            .instrument(span)
            .await;
        let reflection_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        match outcome {
            Ok((text, _thinking, choice_opt, _)) => {
                let raw = choice_opt
                    .map(|c| c.message.content)
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(text);
                parse_scenario_reflection(raw, reflection_ms)
            }
            Err(err) => ScenarioReflection {
                overall_assessment: format!("Reflection turn gateway error: {err}"),
                raw_response: String::new(),
                parsed_structured: false,
                reflection_ms,
                ..ScenarioReflection::default()
            },
        }
    }
}

async fn collect_streaming_turn<G: ModelStreamingGateway>(
    gateway: &G,
    request: ModelGatewayRequest,
) -> Result<
    (
        String,
        String,
        Option<marlin_agent_protocol::ModelGatewayCompletionChoice>,
        bool,
    ),
    ModelGatewayError,
> {
    let mut stream = gateway.stream(request).await?;
    let mut text = String::new();
    let mut thinking = String::new();
    let mut terminal_choice = None;
    let mut tool_requested = false;
    while let Some(item) = stream.next().await {
        match item? {
            ModelStreamEvent::Started { .. } => {}
            ModelStreamEvent::Chunk(chunk) => text.push_str(&chunk.content),
            ModelStreamEvent::Thinking(chunk) => thinking.push_str(&chunk.content),
            ModelStreamEvent::Completed { response } => {
                terminal_choice = response
                    .choices
                    .into_iter()
                    .find(|c| c.message.role == ModelGatewayMessageRole::Assistant);
                tool_requested = false;
                break;
            }
            ModelStreamEvent::ToolRequested { response } => {
                terminal_choice = response
                    .choices
                    .into_iter()
                    .find(|c| c.message.role == ModelGatewayMessageRole::Assistant);
                tool_requested = true;
                break;
            }
        }
    }
    Ok((text, thinking, terminal_choice, tool_requested))
}

fn resolve_workspace_search_cache_paths(repo_root: &Path) -> (PathBuf, PathBuf) {
    let dev_dir = repo_root.join(".data/marlin/search");
    let dev_snap = dev_dir.join("workspace.asp-index.bin");
    let dev_edb = dev_dir.join("workspace.open-edb.v1.json");
    if dev_snap.is_file() || dev_edb.is_file() {
        return (dev_snap, dev_edb);
    }
    (
        default_workspace_snapshot_path(repo_root),
        default_workspace_edb_cache_path(repo_root),
    )
}
