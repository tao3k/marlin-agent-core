// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Post-run reflection prompting, causal trajectory DAG builder, and
//! `AgentHarnessEvidenceGraph` projection helpers for [`super::runner`].

use std::fmt::Write as _;
use std::time::Instant;

use marlin_agent_protocol::{AgentTraceSpanRecord, ModelToolDefinition};

use crate::{
    AgentHarnessEvidenceGraph, AgentHarnessEvidenceGraphEdge, AgentHarnessEvidenceGraphEdgeKind,
    AgentHarnessEvidenceGraphNode, AgentHarnessEvidenceGraphNodeKind,
};

use super::governance::GovernanceDecisionReceipt;
use super::report::DetectedGap;
use super::runner::{
    ScenarioReflection, ToolCallTelemetry, TrajectoryEvent, TrajectoryEventKind,
};
use super::scheme_bridge::{ProjectedHarnessScenario, TemporalCausalityVerdict};

pub(super) fn build_reflection_message(
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

pub(super) fn parse_scenario_reflection(raw: String, reflection_ms: u64) -> ScenarioReflection {
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

pub(super) struct TrajectoryBuilder {
    started: Instant,
    next_pos: usize,
    last_time_pos: usize,
    events: Vec<TrajectoryEvent>,
}

impl TrajectoryBuilder {
    pub(super) fn new(started: Instant) -> Self {
        Self {
            started,
            next_pos: 1,
            last_time_pos: 0,
            events: Vec::new(),
        }
    }

    pub(super) fn push(
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

    pub(super) fn into_events(self) -> Vec<TrajectoryEvent> {
        self.events
    }
}

pub(super) struct ScenarioEvidenceGraphInputs<'a> {
    pub scenario: &'a ProjectedHarnessScenario,
    pub governance_receipts: &'a [GovernanceDecisionReceipt],
    pub tool_calls: &'a [ToolCallTelemetry],
    pub temporal: &'a TemporalCausalityVerdict,
    pub spans: &'a [AgentTraceSpanRecord],
    pub gaps: &'a [DetectedGap],
    pub matched_checkpoints: usize,
    pub total_checkpoints: usize,
}

pub(super) fn build_scenario_evidence_graph(
    input: &ScenarioEvidenceGraphInputs<'_>,
) -> AgentHarnessEvidenceGraph {
    let scenario = input.scenario;
    let intent_id = format!("intent:{}", scenario.id);
    let gov_id = format!("governance:{}", scenario.governance.policy_id);
    let causal_id = format!("temporal-causality:{}", scenario.id);
    let telemetry_id = format!("telemetry:{}", scenario.id);
    let gt_id = format!("ground-truth:{}", scenario.id);

    let denied_writes = input
        .governance_receipts
        .iter()
        .filter(|r| !r.permitted && r.is_write)
        .count();
    let permitted_writes = input
        .governance_receipts
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
            input.governance_receipts.len(),
        )),
        if input.temporal.admitted {
            AgentHarnessEvidenceGraphNode::present(
                &causal_id,
                AgentHarnessEvidenceGraphNodeKind::ProofResult,
                "Bi-Temporal Causality Trajectory Proof",
            )
            .with_detail(format!(
                "status={} replay={} events={} digest={}",
                input.temporal.temporal_status,
                input.temporal.replay_status,
                input.temporal.causal_event_count,
                input.temporal.proof_digest,
            ))
        } else {
            AgentHarnessEvidenceGraphNode::missing(
                &causal_id,
                AgentHarnessEvidenceGraphNodeKind::Counterexample,
                "Bi-Temporal Causality Trajectory Violation",
            )
            .with_detail(format!(
                "status={} replay={} forbidden={:?}",
                input.temporal.temporal_status,
                input.temporal.replay_status,
                input.temporal.forbidden_hits,
            ))
        },
        AgentHarnessEvidenceGraphNode::present(
            &telemetry_id,
            AgentHarnessEvidenceGraphNodeKind::EvidenceFact,
            "OTel TraceRecorder Spans",
        )
        .with_detail(format!(
            "spans={} tool_calls={}",
            input.spans.len(),
            input.tool_calls.len()
        )),
        if input.matched_checkpoints == input.total_checkpoints {
            AgentHarnessEvidenceGraphNode::present(
                &gt_id,
                AgentHarnessEvidenceGraphNodeKind::TestBehavior,
                "Ground-Truth Checkpoints",
            )
            .with_detail(format!(
                "{}/{} matched",
                input.matched_checkpoints, input.total_checkpoints
            ))
        } else {
            AgentHarnessEvidenceGraphNode::missing(
                &gt_id,
                AgentHarnessEvidenceGraphNodeKind::Counterexample,
                "Ground-Truth Checkpoints Incomplete",
            )
            .with_detail(format!(
                "{}/{} matched",
                input.matched_checkpoints, input.total_checkpoints
            ))
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
            if input.matched_checkpoints == input.total_checkpoints {
                AgentHarnessEvidenceGraphEdgeKind::Supports
            } else {
                AgentHarnessEvidenceGraphEdgeKind::Refutes
            },
        ),
    ];

    for (idx, gap) in input.gaps.iter().enumerate() {
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
