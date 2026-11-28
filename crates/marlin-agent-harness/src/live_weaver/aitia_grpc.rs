// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Headless live Weaver tool-calling harness for progressive FunFlow/ASCENT
//! SDLC Git/JJ review over `aitia serve-marlin-grpc` (RFC 0024 / RFC 0026).
//!
//! Reuses Marlin's production read-only `ToolRuntime` implementation
//! ([`AitiaReviewToolHandler`]), loopback gRPC client ([`AitiaSidecarGrpcClient`]),
//! and Magit human-disposition projection ([`AitiaMagitReviewSnapshot`]) from
//! `marlin-agent-tooling`, enforcing fail-closed Cedar read-only governance
//! (`CompiledHarnessGovernanceGate`) and Marlin Scheme Bi-Temporal Causality
//! (`HarnessSchemeBridge`).

use std::path::Path;
use std::time::Instant;

use marlin_agent_protocol::{
    AgentTraceSpanRecord, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayError, ModelGatewayMessage, ModelGatewayMessageRole, ModelGatewayRequest,
    ModelToolChoice, ModelToolDefinition, tool_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::{
    ModelToolOutcome, TokioAgentRuntime, TokioAgentRuntimeBuildRequest, ToolRuntime,
};
use marlin_agent_stream::{ModelStreamEvent, ModelStreamingGateway};
pub use marlin_agent_tooling::{
    AITIA_REVIEW_TOOL_NAME, AitiaGrpcRawResponse, AitiaMagitFinding, AitiaMagitOverlaySummary,
    AitiaMagitReviewSnapshot, AitiaReviewToolHandler, AitiaSidecarGrpcClient,
    ManagedAitiaSidecarServer,
};
use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::Instrument;
use tracing::instrument::WithSubscriber;

use crate::TraceRecorder;

use super::artifacts::TrajectoryBuilder;
use super::config::LiveWeaverRoute;
use super::governance::{
    CompiledHarnessGovernanceGate, GovernanceDecisionReceipt, ProjectedCedarPolicySpec,
    ProjectedHarnessGovernance,
};
use super::runner::{TrajectoryEvent, TrajectoryEventKind, TurnTelemetry};
use super::scheme_bridge::{HarnessSchemeBridge, TemporalCausalityVerdict};

/// Fail-closed read-only Cedar governance policy permitting ONLY the read-only
/// `aitia_review_step` tool (`capability = "review"`, `write_allowed = false`).
#[must_use]
pub fn aitia_review_readonly_governance_policy() -> ProjectedHarnessGovernance {
    ProjectedHarnessGovernance {
        policy_id: "marlin-aitia-review-readonly-v1".to_owned(),
        write_allowed: false,
        allowed_capabilities: vec!["review".to_owned()],
        denied_capabilities: vec![
            "shell".to_owned(),
            "write".to_owned(),
            "edit".to_owned(),
            "mutation".to_owned(),
            "network".to_owned(),
        ],
        allowed_tools: vec![AITIA_REVIEW_TOOL_NAME.to_owned()],
        denied_tools: vec![
            "bash".to_owned(),
            "write_file".to_owned(),
            "edit_file".to_owned(),
        ],
        cedar_policies: vec![
            ProjectedCedarPolicySpec {
                policy_id: "permit-aitia-review-readonly".to_owned(),
                source: r#"permit(principal, action == Marlin::Action::"tool.review", resource == Marlin::Resource::"aitia_review_step") when { !context.is_write };"#
                    .to_owned(),
            },
            ProjectedCedarPolicySpec {
                policy_id: "forbid-any-write-tool".to_owned(),
                source: r#"forbid(principal, action, resource) when { context.is_write };"#
                    .to_owned(),
            },
        ],
        denial_message:
            "Fail-closed Cedar policy: only read-only `aitia_review_step` is permitted; all shell/write tools are forbidden."
                .to_owned(),
    }
}

/// Per-call telemetry recorded for one `aitia_review_step` invocation executed
/// through Marlin's production [`AitiaReviewToolHandler`].
#[derive(Clone, Debug)]
pub struct AitiaReviewCallTelemetry {
    pub round: usize,
    pub call_id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub elapsed_ms: u64,
    pub output_bytes: usize,
    pub status: &'static str,
    pub governance: GovernanceDecisionReceipt,
    pub cas_hit: bool,
    pub rule_packs_applied: Vec<String>,
    pub compact_choices: bool,
    pub quint_overlay_executed: bool,
    pub quint_overlay_passed: bool,
    pub quint_countermodel_verified: bool,
    pub quint_passing_tests: usize,
    pub quint_archived_path: Option<String>,
    pub lean_overlay_executed: bool,
    pub lean_overlay_passed: bool,
    pub lean_proof_clean: bool,
    pub lean_countermodel_verified: bool,
    pub lean_theorem_count: usize,
    pub lean_archived_path: Option<String>,
    pub plan_format: String,
    pub plan_validated: bool,
    pub plan_acyclic: bool,
    pub choice_status: String,
    pub world_count: usize,
    pub recommended_models_count: usize,
    pub dsl_grammar_emitted: bool,
    pub action_authorized: bool,
    pub magit_snapshot: Option<AitiaMagitReviewSnapshot>,
}

/// Complete typed report for a live Weaver LLM tool-calling eval over `aitia_review_step`.
#[derive(Clone, Debug)]
pub struct LiveAitiaReviewReport {
    pub scenario_id: String,
    pub rounds_used: usize,
    pub wall_ms: u64,
    pub turns: Vec<TurnTelemetry>,
    pub tool_calls: Vec<AitiaReviewCallTelemetry>,
    pub governance_receipts: Vec<GovernanceDecisionReceipt>,
    pub trajectory: Vec<TrajectoryEvent>,
    pub temporal_causality: TemporalCausalityVerdict,
    pub telemetry_spans: Vec<AgentTraceSpanRecord>,
    pub final_answer: String,
    pub aborted_reason: Option<String>,
}

/// Run a bounded (`<= 4` rounds) live Weaver LLM tool-calling evaluation against
/// `aitia serve-marlin-grpc` through Marlin's production [`AitiaReviewToolHandler`]
/// (`ToolRuntime`), gated by Cedar read-only governance and verified by Marlin's
/// Scheme Bi-Temporal Causality engine.
#[allow(clippy::too_many_arguments)]
pub async fn run_live_aitia_review_eval(
    route: &LiveWeaverRoute,
    client: &AitiaSidecarGrpcClient,
    scheme_bridge: &HarnessSchemeBridge,
    scenario_id: &str,
    default_repo: &Path,
    external_vault_dir: Option<&Path>,
    user_prompt: &str,
    max_rounds: usize,
) -> Result<LiveAitiaReviewReport, String> {
    let tool_def = client.get_tool_definition()?;
    let governance_gate =
        CompiledHarnessGovernanceGate::compile(aitia_review_readonly_governance_policy())?;
    let recorder = TraceRecorder::new();
    let scenario_span = tracing::info_span!(
        "harness.aitia_review.run",
        scenario.id = scenario_id,
        model.provider = route.endpoint.provider.as_str(),
        model.id = route.endpoint.model.as_str(),
    );

    let handler = AitiaReviewToolHandler::new(
        client.clone(),
        default_repo.to_path_buf(),
        external_vault_dir.map(Path::to_path_buf),
    );
    let (runtime_owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));

    run_live_aitia_review_eval_traced(
        route,
        &handler,
        &runtime_owner,
        scheme_bridge,
        tool_def,
        &governance_gate,
        &recorder,
        scenario_id,
        user_prompt,
        max_rounds.clamp(1, 4),
    )
    .instrument(scenario_span)
    .with_subscriber(recorder.clone())
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_live_aitia_review_eval_traced(
    route: &LiveWeaverRoute,
    handler: &AitiaReviewToolHandler,
    runtime_owner: &TokioAgentRuntime,
    scheme_bridge: &HarnessSchemeBridge,
    tool_def: ModelToolDefinition,
    governance_gate: &CompiledHarnessGovernanceGate,
    recorder: &TraceRecorder,
    scenario_id: &str,
    user_prompt: &str,
    max_rounds: usize,
) -> Result<LiveAitiaReviewReport, String> {
    let started = Instant::now();
    let tool_options = ModelGatewayCompletionOptions {
        tools: Some(vec![tool_def]),
        tool_choice: Some(ModelToolChoice::Auto),
        ..ModelGatewayCompletionOptions::default()
    };

    let mut messages: Vec<ModelGatewayMessage> = vec![user_gateway_message(user_prompt)];
    let mut trajectory = TrajectoryBuilder::new(started);
    let prompt_ev = trajectory.push(
        TrajectoryEventKind::PromptSubmitted,
        Vec::new(),
        format!("scenario={scenario_id}"),
    );
    let mut last_frontier = vec![prompt_ev];

    let mut turns = Vec::new();
    let mut tool_calls = Vec::new();
    let mut governance_receipts = Vec::new();
    let mut final_answer = String::new();
    let mut aborted_reason = None;

    for round in 1..=max_rounds {
        let is_synthesis_turn = max_rounds > 1 && round == max_rounds && !tool_calls.is_empty();
        if is_synthesis_turn {
            messages.push(user_gateway_message(
                "Final turn (no more tool calls). Write your concise, verified SDLC review report directly from the `aitia_review_step` evidence already gathered, including findings, ASCENT world convergence status, Quint/Lean overlay verification & archive paths, and recommended DE formal models.",
            ));
        }

        let turn_started = Instant::now();
        let request = if is_synthesis_turn {
            ModelGatewayRequest::new(route.endpoint.clone(), messages.clone())
        } else {
            ModelGatewayRequest::new(route.endpoint.clone(), messages.clone())
                .with_options(tool_options.clone())
        };
        let stream_res = collect_streaming_turn(route.gateway.as_ref(), request).await;
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
            thinking_verified: true,
        });

        let turn_ev = trajectory.push(
            TrajectoryEventKind::ModelTurnCompleted,
            last_frontier.clone(),
            format!("round={round} tools={}", choice.message.tool_calls.len()),
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
                aborted_reason = Some(format!("round {round} returned empty final answer"));
            } else {
                let _ = trajectory.push(
                    TrajectoryEventKind::FinalAnswerProduced,
                    vec![turn_ev],
                    format!("chars={}", final_answer.chars().count()),
                );
            }
            break;
        }

        messages.push(assistant_msg);
        let mut round_events = Vec::with_capacity(requested_calls.len());

        for call in requested_calls {
            let tool_name = call.name.as_str().to_owned();
            let call_id_str = call.id.as_str().to_owned();
            let declared_cap = if tool_name == AITIA_REVIEW_TOOL_NAME {
                Some("review")
            } else {
                None
            };
            let gov_receipt = governance_gate.evaluate_tool_call(
                scenario_id,
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
                    "tool={tool_name} cap={} cedar={}",
                    gov_receipt.capability, gov_receipt.cedar_decision
                ),
            );
            governance_receipts.push(gov_receipt.clone());

            let req_val = Value::Object(call.arguments.clone());
            if !gov_receipt.permitted {
                let denial = gov_receipt.reason.clone();
                let ev = trajectory.push(
                    TrajectoryEventKind::ToolDenied,
                    vec![gov_ev],
                    format!("tool={tool_name} denied"),
                );
                round_events.push(ev);
                tool_calls.push(empty_call_telemetry(
                    round,
                    call_id_str,
                    tool_name,
                    req_val,
                    0,
                    denial.len(),
                    "denied",
                    gov_receipt,
                ));
                messages.push(tool_gateway_message(call.id, denial));
                continue;
            }

            let call_started = Instant::now();
            let outcome = handler
                .run_tool(call.clone(), runtime_owner.context())
                .await;
            let elapsed_ms = u64::try_from(call_started.elapsed().as_millis()).unwrap_or(u64::MAX);

            match outcome {
                ModelToolOutcome::Completed(body) => {
                    let resp: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    let magit_snapshot = AitiaMagitReviewSnapshot::from_review_response(&resp).ok();
                    let cas_hit = resp["sharedPrefix"]["sharedPrefixCasHit"]
                        .as_bool()
                        .unwrap_or(false);
                    let rule_packs_applied: Vec<String> = resp["sharedPrefix"]["rulePacksApplied"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                    let compact_choices = resp["compactChoices"].as_bool().unwrap_or(false);
                    let quint_overlay_executed = resp["formalOverlayExecution"]["machineExecuted"]
                        .as_bool()
                        .unwrap_or(false);
                    let quint_overlay_passed = resp["formalOverlayExecution"]["passed"]
                        .as_bool()
                        .unwrap_or(false);
                    let quint_countermodel_verified =
                        resp["formalOverlayExecution"]["countermodelVerified"]
                            .as_bool()
                            .unwrap_or(false);
                    let quint_passing_tests = usize::try_from(
                        resp["formalOverlayExecution"]["passingTestCount"]
                            .as_u64()
                            .unwrap_or(0),
                    )
                    .unwrap_or(0);
                    let quint_archived_path = resp["formalOverlayExecution"]["archivedOverlayPath"]
                        .as_str()
                        .map(str::to_owned);
                    let lean_overlay_executed = resp["leanOverlayExecution"]["machineExecuted"]
                        .as_bool()
                        .unwrap_or(false);
                    let lean_overlay_passed = resp["leanOverlayExecution"]["passed"]
                        .as_bool()
                        .unwrap_or(false);
                    let lean_proof_clean = resp["leanOverlayExecution"]["proofClean"]
                        .as_bool()
                        .unwrap_or(false);
                    let lean_countermodel_verified =
                        resp["leanOverlayExecution"]["countermodelVerified"]
                            .as_bool()
                            .unwrap_or(false);
                    let lean_theorem_count = usize::try_from(
                        resp["leanOverlayExecution"]["theoremCount"]
                            .as_u64()
                            .unwrap_or(0),
                    )
                    .unwrap_or(0);
                    let lean_archived_path = resp["leanOverlayExecution"]["archivedOverlayPath"]
                        .as_str()
                        .map(str::to_owned);
                    let plan_format = resp["planValidation"]["planFormat"]
                        .as_str()
                        .unwrap_or("")
                        .to_owned();
                    let plan_validated = resp["planValidation"]["validated"]
                        .as_bool()
                        .unwrap_or(false);
                    let plan_acyclic = resp["planValidation"]["acyclic"].as_bool().unwrap_or(false);
                    let choice_status = resp["ascentChoiceSolution"]["choiceStatus"]
                        .as_str()
                        .unwrap_or("")
                        .to_owned();
                    let world_count = usize::try_from(
                        resp["ascentChoiceSolution"]["worldCount"]
                            .as_u64()
                            .unwrap_or(0),
                    )
                    .unwrap_or(0);
                    let recommended_models_count = resp["recommendedFormalModels"]
                        .as_array()
                        .map_or(0, Vec::len);
                    let dsl_grammar_emitted = resp.get("dslGrammar").is_some();
                    let action_authorized = resp["actionAuthorized"].as_bool().unwrap_or(true);

                    let ev = trajectory.push(
                        TrajectoryEventKind::ToolSearchCompleted,
                        vec![gov_ev],
                        format!(
                            "tool={tool_name} cas={cas_hit} valid={plan_validated} worlds={world_count} ms={elapsed_ms}"
                        ),
                    );
                    round_events.push(ev);
                    tool_calls.push(AitiaReviewCallTelemetry {
                        round,
                        call_id: call_id_str,
                        tool_name,
                        arguments: req_val,
                        elapsed_ms,
                        output_bytes: body.len(),
                        status: "completed",
                        governance: gov_receipt,
                        cas_hit,
                        rule_packs_applied,
                        compact_choices,
                        quint_overlay_executed,
                        quint_overlay_passed,
                        quint_countermodel_verified,
                        quint_passing_tests,
                        quint_archived_path,
                        lean_overlay_executed,
                        lean_overlay_passed,
                        lean_proof_clean,
                        lean_countermodel_verified,
                        lean_theorem_count,
                        lean_archived_path,
                        plan_format,
                        plan_validated,
                        plan_acyclic,
                        choice_status,
                        world_count,
                        recommended_models_count,
                        dsl_grammar_emitted,
                        action_authorized,
                        magit_snapshot,
                    });
                    messages.push(tool_gateway_message(call.id, body));
                }
                ModelToolOutcome::Failed(err_msg) => {
                    let ev = trajectory.push(
                        TrajectoryEventKind::ToolSearchFailed,
                        vec![gov_ev],
                        format!("tool={tool_name} err={err_msg}"),
                    );
                    round_events.push(ev);
                    tool_calls.push(empty_call_telemetry(
                        round,
                        call_id_str,
                        tool_name,
                        req_val,
                        elapsed_ms,
                        err_msg.len(),
                        "failed",
                        gov_receipt,
                    ));
                    messages.push(tool_gateway_message(call.id, err_msg));
                }
            }
        }
        if !round_events.is_empty() {
            last_frontier = round_events;
        }
        if round == max_rounds && final_answer.is_empty() {
            aborted_reason = Some(format!(
                "reached max_rounds budget ({max_rounds}) without final answer"
            ));
        }
    }

    let trajectory_events = trajectory.into_events();
    let temporal_causality =
        scheme_bridge.analyze_trajectory(scenario_id, false, trajectory_events.clone())?;

    Ok(LiveAitiaReviewReport {
        scenario_id: scenario_id.to_owned(),
        rounds_used: turns.len(),
        wall_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        turns,
        tool_calls,
        governance_receipts,
        trajectory: trajectory_events,
        temporal_causality,
        telemetry_spans: recorder.spans(),
        final_answer,
        aborted_reason,
    })
}

#[allow(clippy::too_many_arguments)]
fn empty_call_telemetry(
    round: usize,
    call_id: String,
    tool_name: String,
    arguments: Value,
    elapsed_ms: u64,
    output_bytes: usize,
    status: &'static str,
    governance: GovernanceDecisionReceipt,
) -> AitiaReviewCallTelemetry {
    AitiaReviewCallTelemetry {
        round,
        call_id,
        tool_name,
        arguments,
        elapsed_ms,
        output_bytes,
        status,
        governance,
        cas_hit: false,
        rule_packs_applied: Vec::new(),
        compact_choices: false,
        quint_overlay_executed: false,
        quint_overlay_passed: false,
        quint_countermodel_verified: false,
        quint_passing_tests: 0,
        quint_archived_path: None,
        lean_overlay_executed: false,
        lean_overlay_passed: false,
        lean_proof_clean: false,
        lean_countermodel_verified: false,
        lean_theorem_count: 0,
        lean_archived_path: None,
        plan_format: String::new(),
        plan_validated: false,
        plan_acyclic: false,
        choice_status: String::new(),
        world_count: 0,
        recommended_models_count: 0,
        dsl_grammar_emitted: false,
        action_authorized: false,
        magit_snapshot: None,
    }
}

async fn collect_streaming_turn<G: ModelStreamingGateway>(
    gateway: &G,
    request: ModelGatewayRequest,
) -> Result<(String, String, Option<ModelGatewayCompletionChoice>, bool), ModelGatewayError> {
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
