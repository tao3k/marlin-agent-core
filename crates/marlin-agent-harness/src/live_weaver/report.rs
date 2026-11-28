// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Telemetry parsing, ground-truth checkpoint scoring, automatic gap detection,
//! and Markdown report rendering for live Weaver evaluation suites.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use marlin_agent_protocol::ModelEndpoint;

use super::runner::{LiveScenarioReport, ToolCallTelemetry, TurnTelemetry};
use super::scheme_bridge::{
    ProjectedCheckpoint, ProjectedHarnessScenario, TemporalCausalityVerdict,
};

/// Execution tier / tool kind observed on one model tool call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryTierKind {
    L0Brute,
    L1Ascent,
    L2Dusa,
    Parser,
    Bash,
    Other,
}

impl QueryTierKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::L0Brute => "L0-Brute",
            Self::L1Ascent => "L1-Ascent",
            Self::L2Dusa => "L2-Dusa",
            Self::Parser => "Parser",
            Self::Bash => "Bash",
            Self::Other => "Other",
        }
    }
}

/// Classify a model tool call into its semantic execution tier.
#[must_use]
pub fn classify_tool_call(tool_name: &str, argument: &str) -> QueryTierKind {
    if tool_name == "bash" {
        return QueryTierKind::Bash;
    }
    let trimmed = argument.trim();
    let body = if let Some(after) = trimmed.strip_prefix("```") {
        after
            .split_once('\n')
            .map_or(after, |(_, rest)| rest)
            .trim()
    } else {
        trimmed
    };
    let head = body.strip_prefix('(').map(str::trim_start).unwrap_or(body);
    if head.starts_with("brute-search")
        || head.starts_with("workspace-map")
        || head.starts_with("dir-map")
        || head.starts_with("repo-map")
        || head.starts_with("file-outline")
        || head.starts_with("outline")
        || head.starts_with("read-file")
    {
        QueryTierKind::L0Brute
    } else if head.starts_with("search-choice-program")
        || head.starts_with("relational-choice-program")
        || head.starts_with("solve-placement")
        || head.starts_with("solve-resolve")
    {
        QueryTierKind::L2Dusa
    } else if head.starts_with("ascent")
        || head.starts_with("find-collisions")
        || head.starts_with("find-impact")
        || head.starts_with("find-callers")
        || head.starts_with("find-impls")
        || head.starts_with("find-symbols")
        || head.starts_with("trace-path")
        || head.starts_with('(')
    {
        QueryTierKind::L1Ascent
    } else if head.starts_with("search-selector") {
        QueryTierKind::Parser
    } else {
        QueryTierKind::Other
    }
}

/// Structured header fields extracted from a `workspace_search` receipt.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParsedSearchReceipt {
    pub mode: Option<String>,
    pub total_count: Option<usize>,
    pub returned_count: Option<usize>,
    pub truncated: bool,
    pub edb_coverage: Option<String>,
    pub repairs: Option<String>,
    pub diagnostic: Option<String>,
}

/// Parse standard header lines emitted by L0 `brute-search`, L1 `:gerbil-ascent`,
/// and L2 Dusa receipts.
#[must_use]
pub fn parse_search_receipt(output: &str) -> ParsedSearchReceipt {
    let mut parsed = ParsedSearchReceipt::default();
    for line in output.lines().take(24) {
        if let Some(v) = line.strip_prefix("mode:") {
            parsed.mode = Some(v.trim().to_owned());
        } else if let Some(v) = line.strip_prefix("results:") {
            parsed.total_count = v.trim().parse::<usize>().ok();
        } else if let Some(v) = line.strip_prefix("hits:") {
            parsed.total_count = v.trim().parse::<usize>().ok();
        } else if let Some(v) = line.strip_prefix("dirs:") {
            parsed.total_count = parsed
                .total_count
                .or_else(|| v.trim().parse::<usize>().ok());
        } else if let Some(v) = line.strip_prefix("worlds:") {
            parsed.total_count = v
                .split_whitespace()
                .next()
                .and_then(|tok| tok.parse::<usize>().ok());
        } else if let Some(v) = line.strip_prefix("returned:") {
            parsed.returned_count = v.trim().parse::<usize>().ok();
        } else if let Some(v) = line.strip_prefix("truncated:") {
            parsed.truncated = v.trim() == "true";
        } else if let Some(v) = line
            .strip_prefix("scope_coverage:")
            .or_else(|| line.strip_prefix("edb_coverage:"))
        {
            parsed.edb_coverage = Some(v.trim().to_owned());
        } else if let Some(v) = line.strip_prefix("repairs:") {
            parsed.repairs = Some(v.trim().to_owned());
        } else if let Some(v) = line.strip_prefix("diagnostic:") {
            parsed.diagnostic = Some(v.trim().to_owned());
        }
    }
    parsed
}

/// One automatically classified friction point or architectural gap surfaced by a
/// live Weaver evaluation scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectedGap {
    pub code: &'static str,
    pub detail: String,
}

/// Score a final model answer against case-insensitive ground-truth checkpoints.
#[must_use]
pub fn score_ground_truth(
    answer: &str,
    checkpoints: &[ProjectedCheckpoint],
) -> (usize, Vec<String>) {
    let answer_lower = answer.to_ascii_lowercase();
    let mut matched = 0usize;
    let mut missed = Vec::new();
    for cp in checkpoints {
        let hit = cp
            .any_of
            .iter()
            .any(|needle| answer_lower.contains(&needle.to_ascii_lowercase()));
        if hit {
            matched += 1;
        } else {
            missed.push(cp.label.clone());
        }
    }
    (matched, missed)
}

/// Classify operational, governance, temporal-causality, and semantic gaps
/// observed during one live scenario.
#[must_use]
pub fn detect_scenario_gaps(
    scenario: &ProjectedHarnessScenario,
    rounds_used: usize,
    turns: &[TurnTelemetry],
    tool_calls: &[ToolCallTelemetry],
    temporal: &TemporalCausalityVerdict,
    missed_checkpoints: &[String],
    aborted_reason: Option<&str>,
) -> Vec<DetectedGap> {
    let mut gaps = Vec::new();

    if !temporal.admitted {
        gaps.push(DetectedGap {
            code: "TEMPORAL_CAUSALITY_VIOLATION",
            detail: format!(
                "temporal_status={} replay_status={} forbidden_hits={:?}",
                temporal.temporal_status, temporal.replay_status, temporal.forbidden_hits
            ),
        });
    }

    if !scenario.governance.write_allowed {
        let write_permitted = tool_calls
            .iter()
            .filter(|tc| tc.governance.is_write && tc.governance.permitted)
            .count();
        if write_permitted > 0 {
            gaps.push(DetectedGap {
                code: "GOVERNANCE_WRITE_BREACH",
                detail: format!("{write_permitted} write-capable tool call(s) were permitted"),
            });
        }
    }

    let bash_calls: Vec<&ToolCallTelemetry> = tool_calls
        .iter()
        .filter(|tc| tc.tool_name == "bash")
        .collect();
    if !bash_calls.is_empty() {
        let samples = bash_calls
            .iter()
            .map(|tc| compact_one_line(&tc.argument, 80))
            .collect::<Vec<_>>()
            .join(" | ");
        gaps.push(DetectedGap {
            code: "BASH_FALLBACK",
            detail: format!(
                "model attempted bash {} time(s) (Cedar denied): {samples}",
                bash_calls.len()
            ),
        });
    }

    let is_search_tool = |tc: &ToolCallTelemetry| {
        tc.governance.capability == "search" || tc.tool_name == "workspace_search"
    };
    if scenario.require_workspace_search && !tool_calls.iter().any(is_search_tool) {
        gaps.push(DetectedGap {
            code: "NO_WORKSPACE_SEARCH_USED",
            detail: "model never invoked any read-only search tool".to_owned(),
        });
    }

    for required_tier in &scenario.required_tiers {
        let satisfied = tool_calls.iter().any(|tc| {
            tc.status == "completed"
                && tc.tier.as_str().eq_ignore_ascii_case(required_tier)
                && tc.receipt.total_count.is_some_and(|n| n > 0)
        });
        if !satisfied {
            gaps.push(DetectedGap {
                code: "MISSING_REQUIRED_TIER",
                detail: format!(
                    "scenario requires at least one non-empty completed `{required_tier}` receipt"
                ),
            });
        }
    }

    for tc in tool_calls {
        if tc.status == "failed" {
            gaps.push(DetectedGap {
                code: "SEARCH_QUERY_ERROR",
                detail: format!(
                    "R{} [{}] failed: {}  query={}",
                    tc.round,
                    tc.tier.as_str(),
                    compact_one_line(&tc.error_or_preview, 140),
                    compact_one_line(&tc.argument, 120),
                ),
            });
        } else if tc.status == "completed" && tc.receipt.total_count == Some(0) {
            gaps.push(DetectedGap {
                code: "ZERO_HIT_QUERY",
                detail: format!(
                    "R{} [{}] returned 0 hits (diag={:?}) query={}",
                    tc.round,
                    tc.tier.as_str(),
                    tc.receipt.diagnostic,
                    compact_one_line(&tc.argument, 120),
                ),
            });
        }
        if let Some(repairs) = tc.receipt.repairs.as_deref() {
            let non_shorthand: Vec<&str> = repairs
                .split(',')
                .map(str::trim)
                .filter(|tag| {
                    !tag.is_empty()
                        && !matches!(
                            *tag,
                            "synthesize-head-relation"
                                | "wrap-brute-parens"
                                | "wrap-outline-parens"
                                | "wrap-read-file-parens"
                        )
                })
                .collect();
            if !non_shorthand.is_empty() {
                gaps.push(DetectedGap {
                    code: "AST_REPAIR_TRIGGERED",
                    detail: format!(
                        "R{} [{}] required auto-repair ({}) query={}",
                        tc.round,
                        tc.tier.as_str(),
                        non_shorthand.join(", "),
                        compact_one_line(&tc.argument, 120),
                    ),
                });
            }
        }
    }

    if rounds_used > scenario.target_max_rounds {
        gaps.push(DetectedGap {
            code: "MULTI_ROUND_OVERHEAD",
            detail: format!(
                "took {rounds_used} rounds (target <= {})",
                scenario.target_max_rounds
            ),
        });
    }

    if turns.iter().any(|t| !t.thinking_verified) {
        gaps.push(DetectedGap {
            code: "CONTINUATION_UNVERIFIED",
            detail: "observed thinking failed Anthropic continuation verification".to_owned(),
        });
    }

    if turns
        .iter()
        .any(|t| matches!(t.finish_reason.as_deref(), Some("max_tokens" | "length")))
    {
        gaps.push(DetectedGap {
            code: "OUTPUT_TRUNCATED",
            detail: "provider terminated a turn with max_tokens/length".to_owned(),
        });
    }

    if !missed_checkpoints.is_empty() {
        gaps.push(DetectedGap {
            code: "MISSING_GROUND_TRUTH",
            detail: format!("missed checkpoints: {}", missed_checkpoints.join(", ")),
        });
    }

    if let Some(reason) = aborted_reason {
        gaps.push(DetectedGap {
            code: "LOOP_ABORTED",
            detail: reason.to_owned(),
        });
    }

    gaps
}

pub(crate) fn compact_one_line(raw: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut prev_ws = false;
    let mut count = 0usize;
    for ch in raw.chars() {
        let mapped = if ch.is_whitespace() || ch.is_control() {
            ' '
        } else {
            ch
        };
        if mapped == ' ' {
            if prev_ws || out.is_empty() {
                continue;
            }
            prev_ws = true;
        } else {
            prev_ws = false;
        }
        if count >= max_chars {
            out.push('…');
            break;
        }
        out.push(mapped);
        count += 1;
    }
    out.trim_end().to_owned()
}

/// Render a full structured Markdown scorecard with Governance, OTel Telemetry,
/// Causal Trajectory, Bi-Temporal Causality Proof, EvidenceGraph, and Gaps.
#[must_use]
pub fn render_live_eval_markdown_report(
    suite_id: &str,
    endpoint: &ModelEndpoint,
    indexed_files: usize,
    hydrated_files: usize,
    reports: &[LiveScenarioReport],
) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Marlin Live Weaver Evaluation Report\n");
    let _ = writeln!(md, "- **Scheme Suite**: `{suite_id}`");
    let _ = writeln!(
        md,
        "- **Model**: `{}/{}`",
        endpoint.provider.as_str(),
        endpoint.model.as_str()
    );
    let _ = writeln!(
        md,
        "- **Workspace Index**: `{indexed_files}` indexed files, `{hydrated_files}` Open EDB hydrated files\n"
    );
    let _ = writeln!(
        md,
        "| Scenario | Rounds | Search Tools (L0/L1/L2) | `bash` Denied | Errors | 0-Hits | Causal Proof | Spans | Ground Truth | Wall (ms) | Gaps |"
    );
    let _ = writeln!(md, "|---|---:|---|---:|---:|---:|---|---:|---|---:|---:|");

    for r in reports {
        let l0 = r
            .tool_calls
            .iter()
            .filter(|c| c.tier == QueryTierKind::L0Brute)
            .count();
        let l1 = r
            .tool_calls
            .iter()
            .filter(|c| c.tier == QueryTierKind::L1Ascent)
            .count();
        let l2 = r
            .tool_calls
            .iter()
            .filter(|c| c.tier == QueryTierKind::L2Dusa)
            .count();
        let ws_total = r
            .tool_calls
            .iter()
            .filter(|c| c.governance.capability == "search" || c.tool_name == "workspace_search")
            .count();
        let bash_n = r
            .tool_calls
            .iter()
            .filter(|c| c.tool_name == "bash")
            .count();
        let err_n = r.tool_calls.iter().filter(|c| c.status == "failed").count();
        let zero_n = r
            .tool_calls
            .iter()
            .filter(|c| c.status == "completed" && c.receipt.total_count == Some(0))
            .count();
        let causal_badge = if r.temporal_causality.admitted {
            format!("ADMITTED ({} ev)", r.temporal_causality.causal_event_count)
        } else {
            format!("REJECTED ({:?})", r.temporal_causality.forbidden_hits)
        };
        let _ = writeln!(
            md,
            "| `{}` | {}/{} | {} ({}/{}/{}) | {} | {} | {} | {} | {} | {}/{} | {} | {} |",
            r.scenario_id,
            r.rounds_used,
            r.target_max_rounds,
            ws_total,
            l0,
            l1,
            l2,
            bash_n,
            err_n,
            zero_n,
            causal_badge,
            r.telemetry_spans.len(),
            r.matched_checkpoints,
            r.total_checkpoints,
            r.wall_ms,
            r.gaps.len(),
        );
    }

    for r in reports {
        let _ = writeln!(md, "\n## Scenario `{}` — {}\n", r.scenario_id, r.title);
        let _ = writeln!(md, "> {}\n", r.message);
        let _ = writeln!(
            md,
            "- Governance: `{}` (`write_allowed={}`, Cedar evaluations=`{}`, denied=`{}`)",
            r.governance_policy_id,
            r.write_allowed,
            r.governance_receipts.len(),
            r.governance_receipts
                .iter()
                .filter(|g| !g.permitted)
                .count(),
        );
        let _ = writeln!(
            md,
            "- Temporal Causality: `admitted={}` (`status={}`, `replay={}`, `events={}`, `forbidden={:?}`, `digest={}`)",
            r.temporal_causality.admitted,
            r.temporal_causality.temporal_status,
            r.temporal_causality.replay_status,
            r.temporal_causality.causal_event_count,
            r.temporal_causality.forbidden_hits,
            r.temporal_causality.proof_digest,
        );
        let eg = r.evidence_graph.summary();
        let _ = writeln!(
            md,
            "- EvidenceGraph: `{}` nodes (`{}` missing, `{}` counterexamples), `{}` edges",
            eg.nodes, eg.missing_nodes, eg.counterexamples, eg.edges,
        );
        let mut span_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for s in &r.telemetry_spans {
            *span_counts.entry(s.name.as_str()).or_insert(0) += 1;
        }
        let span_summary = span_counts
            .into_iter()
            .map(|(name, n)| format!("{name}={n}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            md,
            "- OTel Spans ({}): `{span_summary}`",
            r.telemetry_spans.len()
        );
        let _ = writeln!(
            md,
            "- Rounds: `{}`, Wall: `{}ms`, Ground truth: `{}/{}` (missed: `{}`)",
            r.rounds_used,
            r.wall_ms,
            r.matched_checkpoints,
            r.total_checkpoints,
            if r.missed_checkpoints.is_empty() {
                "none".to_owned()
            } else {
                r.missed_checkpoints.join(", ")
            }
        );
        if let Some(reason) = r.aborted_reason.as_deref() {
            let _ = writeln!(md, "- **Aborted**: `{reason}`");
        }
        let _ = writeln!(md, "\n### Turn Telemetry");
        for t in &r.turns {
            let _ = writeln!(
                md,
                "- R{}: `{}ms`, thinking={}B, text={}B, tools={}, finish={:?}, thinking_verified={}",
                t.round,
                t.turn_ms,
                t.thinking_bytes,
                t.text_bytes,
                t.tool_calls,
                t.finish_reason,
                t.thinking_verified,
            );
        }
        let _ = writeln!(md, "\n### Tool Calls & Governance Receipts");
        if r.tool_calls.is_empty() {
            let _ = writeln!(md, "- (none)");
        }
        for tc in &r.tool_calls {
            let _ = writeln!(
                md,
                "- R{} `{}` [{}] `{}` | gov=`{}` (cap=`{}`, write={}, policies={:?}) | `{}ms`, `{}B`, count={:?}, ret={:?}, trunc={}, cov={:?}, repairs={:?}\n  - arg: `{}`\n  - out: `{}`",
                tc.round,
                tc.tool_name,
                tc.tier.as_str(),
                tc.status,
                tc.governance.cedar_decision,
                tc.governance.capability,
                tc.governance.is_write,
                tc.governance.matched_policy_ids,
                tc.elapsed_ms,
                tc.output_bytes,
                tc.receipt.total_count,
                tc.receipt.returned_count,
                tc.receipt.truncated,
                tc.receipt.edb_coverage,
                tc.receipt.repairs,
                compact_one_line(&tc.argument, 240),
                compact_one_line(&tc.error_or_preview, 240),
            );
        }
        let _ = writeln!(
            md,
            "\n### Causal Trajectory DAG ({} events)",
            r.trajectory.len()
        );
        for ev in &r.trajectory {
            let _ = writeln!(
                md,
                "- `{}` [{}] pos={} t=({}, {}) parents={:?} — {}",
                ev.event_id,
                ev.runtime_symbol(),
                ev.logical_pos,
                ev.valid_pos,
                ev.knowledge_pos,
                ev.parent_ids,
                ev.detail,
            );
        }
        let _ = writeln!(md, "\n### Detected Gaps");
        if r.gaps.is_empty() {
            let _ = writeln!(md, "- None");
        } else {
            for g in &r.gaps {
                let _ = writeln!(md, "- `[{}]` {}", g.code, g.detail);
            }
        }
        let _ = writeln!(
            md,
            "\n### Final Answer Excerpt ({} chars)\n\n```\n{}\n```",
            r.final_answer.chars().count(),
            r.final_answer.chars().take(1500).collect::<String>(),
        );
        if let Some(ref refl) = r.reflection {
            let _ = writeln!(
                md,
                "\n### Model Reflection (Awaiting Human Audit — `parsed={}`, `{}ms`)\n",
                refl.parsed_structured, refl.reflection_ms
            );
            let _ = writeln!(md, "> {}\n", refl.overall_assessment.replace('\n', " "));
            if !refl.items.is_empty() {
                let _ = writeln!(
                    md,
                    "| Round | Tier | Symptom | Model Root-Cause Analysis | Attribution | Proposed DSL/Repair Improvement | Human Audit Verdict |"
                );
                let _ = writeln!(md, "|---:|---|---|---|---|---|---|");
                for item in &refl.items {
                    let _ = writeln!(
                        md,
                        "| R{} | {} | {} | {} | `{}` | {} | `[ ] Pending` |",
                        item.round,
                        escape_md_cell(&item.tier),
                        escape_md_cell(&item.symptom),
                        escape_md_cell(&item.why_i_wrote_it_wrong),
                        escape_md_cell(&item.attribution),
                        escape_md_cell(&item.proposed_improvement),
                    );
                }
            }
            if !refl.top_dsl_design_recommendations.is_empty() {
                let _ = writeln!(md, "\n**Top DSL Design Recommendations (Model-Proposed):**");
                for rec in &refl.top_dsl_design_recommendations {
                    let _ = writeln!(md, "- {}", rec.replace('\n', " "));
                }
            }
            if !refl.parsed_structured && !refl.raw_response.trim().is_empty() {
                let _ = writeln!(
                    md,
                    "\nRaw reflection output:\n```\n{}\n```",
                    refl.raw_response.chars().take(2000).collect::<String>()
                );
            }
        }
    }

    md
}

fn escape_md_cell(s: &str) -> String {
    s.replace('\n', " ").replace('|', "\\|")
}
