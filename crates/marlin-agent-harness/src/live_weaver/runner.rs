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
use std::fmt::Write as _;
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
    WorkspaceIndexConfig, default_workspace_edb_cache_path, default_workspace_snapshot_path,
    hydrate_workspace_open_edb_org_and_cache, open_or_build_workspace_index_safe,
};
use marlin_agent_stream::{
    ModelStreamEvent, ModelStreamingGateway, matches_observed_anthropic_thinking,
};
use marlin_agent_tooling::{
    ResidentSearchToolHandler, SharedSearchIndexSlot, catalog_from_declarations_with_search,
};
use tokio_stream::StreamExt;
use tracing::Instrument;
use tracing::instrument::WithSubscriber;

use crate::{
    AgentHarnessEvidenceGraph, AgentHarnessEvidenceGraphEdge, AgentHarnessEvidenceGraphEdgeKind,
    AgentHarnessEvidenceGraphNode, AgentHarnessEvidenceGraphNodeKind, TraceRecorder,
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
    search_handlers: BTreeMap<String, Arc<ResidentSearchToolHandler>>,
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
        let slot = SharedSearchIndexSlot::with_sync_debounce(
            Some(shared_index),
            Duration::from_secs(3600),
        );
        slot.attach_dsl_runner(scheme_bridge.search_dsl_runner());

        let search_handlers: BTreeMap<String, Arc<ResidentSearchToolHandler>> = declarations
            .iter()
            .filter(|decl| decl.capability() == ModelToolCapability::Search)
            .map(|decl| {
                let name = decl.definition().name.as_str().to_owned();
                let handler = Arc::new(ResidentSearchToolHandler::new(
                    repo_root.to_path_buf(),
                    decl.clone(),
                    slot.clone(),
                ));
                (name, handler)
            })
            .collect();
        if search_handlers.is_empty() {
            return Err(
                "Scheme ToolingCatalogValue does not declare any `search` capability tools"
                    .to_owned(),
            );
        }

        let catalog =
            catalog_from_declarations_with_search(declarations, repo_root.to_path_buf(), slot)
                .map_err(|err| format!("failed to build ModelToolCatalog: {err:?}"))?;
        let offer = catalog.offer();
        let offered_definitions = offer.definitions();
        let tool_options = offer.completion_options();

        let (runtime_owner, _events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(16),
        );

        Ok(Self {
            scheme_bridge: scheme_bridge.clone(),
            route,
            budget,
            tool_options,
            offered_definitions,
            tool_capabilities,
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
            let mut round_tool_events = Vec::with_capacity(requested_calls.len());
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

                let call_started = Instant::now();
                let (status, result_body, receipt, preview, exec_kind) = if !gov_receipt.permitted {
                    let actionable_denial =
                        super::governance::format_actionable_denial_message(
                            &gov_receipt.reason,
                            &tool_name,
                            &argument,
                        );
                    (
                        "denied",
                        actionable_denial.clone(),
                        ParsedSearchReceipt::default(),
                        actionable_denial,
                        TrajectoryEventKind::ToolDenied,
                    )
                } else if let Some(handler) = self.search_handlers.get(tool_name.as_str()) {
                    let exec_span = tracing::info_span!(
                        "runtime.tool.execute",
                        tool.name = tool_name.as_str(),
                        tool.call_id = call_id_str.as_str(),
                        tool.tier = tier.as_str(),
                    );
                    match handler
                        .run_tool(call.clone(), self.runtime_owner.context())
                        .instrument(exec_span)
                        .await
                    {
                        ModelToolOutcome::Completed(body) => {
                            let receipt = parse_search_receipt(&body);
                            // Skip noisy hash/timing boilerplate so both the report and the
                            // post-run reflection turn always see status, combinatorics,
                            // partial witnesses, repairs, canonical_query, and diagnostics.
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
                            (
                                "completed",
                                body,
                                receipt,
                                preview,
                                TrajectoryEventKind::ToolSearchCompleted,
                            )
                        }
                        ModelToolOutcome::Failed(err) => {
                            let msg = format!("{tool_name} error: {err}");
                            (
                                "failed",
                                msg,
                                ParsedSearchReceipt::default(),
                                err,
                                TrajectoryEventKind::ToolSearchFailed,
                            )
                        }
                    }
                } else {
                    let denied = format!(
                        "Fail-closed harness refused unimplemented tool `{tool_name}` even after policy check."
                    );
                    (
                        "denied",
                        denied.clone(),
                        ParsedSearchReceipt::default(),
                        denied,
                        TrajectoryEventKind::ToolDenied,
                    )
                };

                let elapsed_ms =
                    u64::try_from(call_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let tool_ev = trajectory.push(
                    exec_kind,
                    vec![gov_ev],
                    format!("tool={tool_name} status={status} ms={elapsed_ms}"),
                );
                round_tool_events.push(tool_ev);

                let output_bytes = result_body.len();
                tool_calls.push(ToolCallTelemetry {
                    round,
                    call_id: call_id_str,
                    tool_name,
                    argument,
                    tier,
                    status,
                    elapsed_ms,
                    output_bytes,
                    receipt,
                    governance: gov_receipt,
                    error_or_preview: preview,
                });
                messages.push(tool_gateway_message(call.id, result_body));

                if total_calls >= max_tool_calls {
                    budget_hit_note = Some(format!("reached max_tool_calls budget ({max_tool_calls})"));
                }
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
        let evidence_graph = build_scenario_evidence_graph(
            scenario,
            &governance_receipts,
            &tool_calls,
            &temporal_causality,
            &telemetry_spans,
            &gaps,
            matched_checkpoints,
            scenario.checkpoints.len(),
        );

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

fn build_reflection_message(
    scenario: &ProjectedHarnessScenario,
    offered_definitions: &[ModelToolDefinition],
    rounds_used: usize,
    tool_calls: &[ToolCallTelemetry],
    gaps: &[DetectedGap],
) -> String {
    let mut ledger = String::new();
    let _ = writeln!(
        ledger,
        "Perform a rigorous, honest post-mortem reflection on your own Search DSL usage below. Base every claim strictly on the declared tool catalog and observed tool call log; NEVER claim a capability is missing if it was already offered in the declared tools below. Output ONLY a single valid JSON object matching the schema at the end.\n"
    );
    let _ = writeln!(ledger, "Declared Tools Available During This Run:");
    for def in offered_definitions {
        let first_line = def.description.lines().next().unwrap_or("");
        let _ = writeln!(ledger, "- `{}`: {}", def.name.as_str(), first_line);
    }
    let _ = writeln!(ledger, "\nScenario `{}` — {}", scenario.id, scenario.title);
    let _ = writeln!(ledger, "User message: {}", scenario.message);
    let _ = writeln!(
        ledger,
        "Observed execution: rounds={rounds_used} (target<={}), total_tool_calls={}",
        scenario.target_max_rounds,
        tool_calls.len()
    );
    let _ = writeln!(ledger, "\nObserved Tool Calls (chronological):");
    for (idx, tc) in tool_calls.iter().enumerate() {
        let _ = writeln!(
            ledger,
            "{}. R{} `{}` [{}] status={} gov={} count={:?} repairs={:?}\n   query: {}\n   output_or_error: {}",
            idx + 1,
            tc.round,
            tc.tool_name,
            tc.tier.as_str(),
            tc.status,
            tc.governance.cedar_decision,
            tc.receipt.total_count,
            tc.receipt.repairs,
            tc.argument,
            tc.error_or_preview.chars().take(900).collect::<String>(),
        );
    }
    let _ = writeln!(ledger, "\nObserved Automated Gaps:");
    if gaps.is_empty() {
        let _ = writeln!(ledger, "- none");
    } else {
        for g in gaps {
            let _ = writeln!(ledger, "- [{}] {}", g.code, g.detail);
        }
    }
    let _ = write!(
        ledger,
        "\nNow reflect critically in English on:\n\
1. Every failed query, auto-repaired query, 0-hit query, or multi-round inefficiency above: WHY did you write it that way, and did the Search DSL / repair hook / diagnostic help you fix it immediately?\n\
2. Systemic Search DSL, repair-hook, diagnostic, or EDB schema design improvements (NOT tiny one-off patches) that would let a fast model write complex L0/L1/L2 queries right on Turn 1 and finish in <=4 rounds.\n\n\
Return ONLY valid JSON with this exact schema:\n\
{{\n\
  \"overall_assessment\": \"1-3 sentences summarizing DSL ergonomics & self-repair experience\",\n\
  \"items\": [\n\
    {{\n\
      \"round\": 1,\n\
      \"tier\": \"L0-Brute | L1-Ascent | L2-Dusa | Bash\",\n\
      \"symptom\": \"concise description of the error, auto-repair, or 0-hit\",\n\
      \"why_i_wrote_it_wrong\": \"honest root cause of why you generated that syntax/query\",\n\
      \"attribution\": \"tool_doc_gap | dsl_syntax_ergonomics | repair_hook_gap | edb_schema_or_coverage_gap | model_mistake\",\n\
      \"proposed_improvement\": \"concrete DSL grammar, repair pass, diagnostic, or schema improvement\"\n\
    }}\n\
  ],\n\
  \"top_dsl_design_recommendations\": [\"systemic design recommendation 1\", \"recommendation 2\"]\n\
}}\n"
    );
    ledger
}

fn parse_scenario_reflection(raw: String, reflection_ms: u64) -> ScenarioReflection {
    let trimmed = raw.trim();
    let unfenced = if let Some(after_open) = trimmed.strip_prefix("```") {
        let body = after_open
            .split_once('\n')
            .map_or(after_open, |(_, rest)| rest);
        body.trim().trim_end_matches("```").trim()
    } else {
        trimmed
    };
    let candidate = if unfenced.starts_with('{') && unfenced.ends_with('}') {
        unfenced
    } else if let (Some(start), Some(end)) = (unfenced.find('{'), unfenced.rfind('}'))
        && start < end
    {
        &unfenced[start..=end]
    } else {
        unfenced
    };

    match serde_json::from_str::<ScenarioReflection>(candidate) {
        Ok(mut parsed) => {
            parsed.raw_response = raw;
            parsed.parsed_structured = true;
            parsed.reflection_ms = reflection_ms;
            parsed
        }
        Err(_) => ScenarioReflection {
            overall_assessment: "Model returned non-JSON reflection (preserved verbatim below)."
                .to_owned(),
            raw_response: raw,
            parsed_structured: false,
            reflection_ms,
            ..ScenarioReflection::default()
        },
    }
}

struct TrajectoryBuilder {
    started: Instant,
    next_pos: usize,
    last_time_pos: usize,
    events: Vec<TrajectoryEvent>,
}

impl TrajectoryBuilder {
    fn new(started: Instant) -> Self {
        Self {
            started,
            next_pos: 1,
            last_time_pos: 0,
            events: Vec::new(),
        }
    }

    fn push(
        &mut self,
        kind: TrajectoryEventKind,
        parent_ids: Vec<String>,
        detail: String,
    ) -> String {
        let pos = self.next_pos;
        self.next_pos += 1;
        let elapsed_ms = usize::try_from(self.started.elapsed().as_millis()).unwrap_or(usize::MAX);
        // Keep bi-temporal valid/knowledge coordinates strictly monotonic.
        let coord = elapsed_ms.max(self.last_time_pos + 1);
        self.last_time_pos = coord;
        let event_id = format!("ev-{pos}");
        self.events.push(TrajectoryEvent {
            event_id: event_id.clone(),
            kind,
            logical_pos: pos,
            parent_ids,
            valid_pos: coord,
            knowledge_pos: coord,
            detail,
        });
        event_id
    }

    fn into_events(self) -> Vec<TrajectoryEvent> {
        self.events
    }
}

#[allow(clippy::too_many_arguments)]
fn build_scenario_evidence_graph(
    scenario: &ProjectedHarnessScenario,
    governance_receipts: &[GovernanceDecisionReceipt],
    tool_calls: &[ToolCallTelemetry],
    temporal: &TemporalCausalityVerdict,
    spans: &[AgentTraceSpanRecord],
    gaps: &[DetectedGap],
    matched_checkpoints: usize,
    total_checkpoints: usize,
) -> AgentHarnessEvidenceGraph {
    let intent_id = format!("intent:{}", scenario.id);
    let gov_id = format!("governance:{}", scenario.governance.policy_id);
    let causal_id = format!("temporal-causality:{}", scenario.id);
    let telemetry_id = format!("telemetry:{}", scenario.id);
    let gt_id = format!("ground-truth:{}", scenario.id);

    let denied_writes = governance_receipts
        .iter()
        .filter(|r| !r.permitted && r.is_write)
        .count();
    let permitted_writes = governance_receipts
        .iter()
        .filter(|r| r.permitted && r.is_write)
        .count();

    let mut nodes = vec![
        AgentHarnessEvidenceGraphNode::present(
            &intent_id,
            AgentHarnessEvidenceGraphNodeKind::HumanIntent,
            scenario.title.clone(),
        )
        .with_detail(format!("scenario_id={}", scenario.id)),
        AgentHarnessEvidenceGraphNode::present(
            &gov_id,
            AgentHarnessEvidenceGraphNodeKind::ReviewJudgment,
            format!("Cedar governance `{}`", scenario.governance.policy_id),
        )
        .with_detail(format!(
            "write_allowed={}, total_evals={}, denied_writes={denied_writes}, permitted_writes={permitted_writes}",
            scenario.governance.write_allowed,
            governance_receipts.len(),
        )),
        if temporal.admitted {
            AgentHarnessEvidenceGraphNode::present(
                &causal_id,
                AgentHarnessEvidenceGraphNodeKind::ProofResult,
                "Bi-Temporal Causality Trajectory Proof",
            )
            .with_detail(format!(
                "status={} replay={} events={} digest={}",
                temporal.temporal_status,
                temporal.replay_status,
                temporal.causal_event_count,
                temporal.proof_digest,
            ))
        } else {
            AgentHarnessEvidenceGraphNode::missing(
                &causal_id,
                AgentHarnessEvidenceGraphNodeKind::Counterexample,
                "Bi-Temporal Causality Trajectory Violation",
            )
            .with_detail(format!(
                "status={} replay={} forbidden={:?}",
                temporal.temporal_status, temporal.replay_status, temporal.forbidden_hits,
            ))
        },
        AgentHarnessEvidenceGraphNode::present(
            &telemetry_id,
            AgentHarnessEvidenceGraphNodeKind::EvidenceFact,
            "OTel TraceRecorder Spans",
        )
        .with_detail(format!(
            "spans={} tool_calls={}",
            spans.len(),
            tool_calls.len()
        )),
        if matched_checkpoints == total_checkpoints {
            AgentHarnessEvidenceGraphNode::present(
                &gt_id,
                AgentHarnessEvidenceGraphNodeKind::TestBehavior,
                "Ground-Truth Checkpoints",
            )
            .with_detail(format!("{matched_checkpoints}/{total_checkpoints} matched"))
        } else {
            AgentHarnessEvidenceGraphNode::missing(
                &gt_id,
                AgentHarnessEvidenceGraphNodeKind::Counterexample,
                "Ground-Truth Checkpoints Incomplete",
            )
            .with_detail(format!("{matched_checkpoints}/{total_checkpoints} matched"))
        },
    ];

    let mut edges = vec![
        AgentHarnessEvidenceGraphEdge::new(
            &intent_id,
            &gov_id,
            AgentHarnessEvidenceGraphEdgeKind::Requires,
        ),
        AgentHarnessEvidenceGraphEdge::new(
            &gov_id,
            &causal_id,
            AgentHarnessEvidenceGraphEdgeKind::Proves,
        ),
        AgentHarnessEvidenceGraphEdge::new(
            &telemetry_id,
            &intent_id,
            AgentHarnessEvidenceGraphEdgeKind::Observes,
        ),
        AgentHarnessEvidenceGraphEdge::new(
            &gt_id,
            &intent_id,
            if matched_checkpoints == total_checkpoints {
                AgentHarnessEvidenceGraphEdgeKind::Supports
            } else {
                AgentHarnessEvidenceGraphEdgeKind::Refutes
            },
        ),
    ];

    for (idx, gap) in gaps.iter().enumerate() {
        let gap_node_id = format!("gap:{}:{idx}", scenario.id);
        nodes.push(
            AgentHarnessEvidenceGraphNode::missing(
                &gap_node_id,
                AgentHarnessEvidenceGraphNodeKind::Counterexample,
                gap.code,
            )
            .with_detail(gap.detail.clone()),
        );
        edges.push(AgentHarnessEvidenceGraphEdge::new(
            &gap_node_id,
            &intent_id,
            AgentHarnessEvidenceGraphEdgeKind::Refutes,
        ));
    }

    let mut graph = AgentHarnessEvidenceGraph::new(format!("live-weaver:{}", scenario.id));
    graph.nodes = nodes;
    graph.edges = edges;
    graph
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
