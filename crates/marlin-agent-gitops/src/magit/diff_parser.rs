// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded unified diff parser, exact hunk patch builder (`magit-apply.el`),
//! and honest multi-source evidence-chain fusion (Lambda Aitia gRPC findings +
//! Formal Model linkage + Human/LLM joint audit verdicts).

use std::collections::BTreeMap;

use super::{
    aitia_client::AitiaLiveReport,
    types::{
        MagitAuditConcern, MagitAuditRisk, MagitDiffHunk, MagitDiffLine, MagitDiffLineKind,
        MagitEvidenceItem, MagitEvidenceSource, MagitFileSection, MagitFormalStatus,
        MagitJointAuditRecord,
    },
};

const MAX_HUNKS_PER_FILE: usize = 48;
const MAX_LINES_PER_HUNK: usize = 160;

/// Parse standard unified Git/Jujutsu diff output into structured per-file hunks
/// with structural concern grouping (initial risk is `Unassessed` until real
/// Lambda Aitia + Formal + Human/LLM evidence is fused via `fuse_hunk_evidence`).
pub fn parse_unified_diff(
    raw: &str,
    section: MagitFileSection,
) -> BTreeMap<String, Vec<MagitDiffHunk>> {
    let mut out: BTreeMap<String, Vec<MagitDiffHunk>> = BTreeMap::new();
    let mut cur_file: Option<String> = None;
    let mut cur_hunk: Option<PendingHunk> = None;
    let mut hunk_ordinal: usize = 0;

    let flush = |cur_hunk: &mut Option<PendingHunk>,
                 out: &mut BTreeMap<String, Vec<MagitDiffHunk>>| {
        if let Some(pending) = cur_hunk.take() {
            let entry = out.entry(pending.file_path.clone()).or_default();
            if entry.len() < MAX_HUNKS_PER_FILE {
                let concern = classify_structural_concern(&pending.file_path);
                let structural_item = MagitEvidenceItem {
                    source: MagitEvidenceSource::Structural,
                    verdict: "diffstat".to_owned(),
                    rule_or_actor: concern.label().to_owned(),
                    summary: format!(
                        "Structural diffstat (+{}/-{}) on {} surface",
                        pending.added,
                        pending.removed,
                        concern.label()
                    ),
                    line: Some(pending.new_start),
                };
                entry.push(MagitDiffHunk {
                    hunk_id: pending.hunk_id,
                    file_path: pending.file_path,
                    header: pending.header,
                    old_start: pending.old_start,
                    old_count: pending.old_count,
                    new_start: pending.new_start,
                    new_count: pending.new_count,
                    added: pending.added,
                    removed: pending.removed,
                    risk: MagitAuditRisk::Unassessed,
                    concern,
                    formal_status: MagitFormalStatus::None,
                    risk_reason: format!(
                        "Awaiting Lambda Aitia / Human-AI evidence (+{}/-{})",
                        pending.added, pending.removed
                    ),
                    evidence: vec![structural_item],
                    lines: pending.lines,
                });
            }
        }
    };

    for raw_line in raw.lines() {
        if let Some(rest) = raw_line.strip_prefix("diff --git ") {
            flush(&mut cur_hunk, &mut out);
            hunk_ordinal = 0;
            cur_file = extract_diff_git_path(rest);
            continue;
        }
        if let Some(rest) = raw_line.strip_prefix("+++ ") {
            if let Some(clean) = normalize_diff_header_path(rest) {
                cur_file = Some(clean);
            }
            continue;
        }
        if raw_line.starts_with("--- ")
            || raw_line.starts_with("index ")
            || raw_line.starts_with("new file mode")
            || raw_line.starts_with("deleted file mode")
            || raw_line.starts_with("old mode")
            || raw_line.starts_with("new mode")
            || raw_line.starts_with("similarity index")
            || raw_line.starts_with("rename from")
            || raw_line.starts_with("rename to")
            || raw_line.starts_with("Binary files ")
        {
            continue;
        }
        if raw_line.starts_with("@@ ") {
            flush(&mut cur_hunk, &mut out);
            let Some(file_path) = cur_file.clone() else {
                continue;
            };
            hunk_ordinal += 1;
            let (old_start, old_count, new_start, new_count) = parse_hunk_ranges(raw_line);
            cur_hunk = Some(PendingHunk {
                hunk_id: format!(
                    "{}:{}:h{}:L{}",
                    section.as_str(),
                    file_path,
                    hunk_ordinal,
                    new_start
                ),
                file_path,
                header: raw_line.to_owned(),
                old_start,
                old_count,
                new_start,
                new_count,
                old_cursor: old_start,
                new_cursor: new_start,
                added: 0,
                removed: 0,
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(pending) = cur_hunk.as_mut() {
            if raw_line.starts_with("\\ No newline at end of file") {
                continue;
            }
            if let Some(text) = raw_line.strip_prefix('+') {
                pending.added += 1;
                let line_no = pending.new_cursor;
                pending.new_cursor += 1;
                if pending.lines.len() < MAX_LINES_PER_HUNK {
                    pending.lines.push(MagitDiffLine {
                        kind: MagitDiffLineKind::Added,
                        old_line: None,
                        new_line: Some(line_no),
                        text: text.to_owned(),
                    });
                }
            } else if let Some(text) = raw_line.strip_prefix('-') {
                pending.removed += 1;
                let line_no = pending.old_cursor;
                pending.old_cursor += 1;
                if pending.lines.len() < MAX_LINES_PER_HUNK {
                    pending.lines.push(MagitDiffLine {
                        kind: MagitDiffLineKind::Removed,
                        old_line: Some(line_no),
                        new_line: None,
                        text: text.to_owned(),
                    });
                }
            } else {
                let text = raw_line.strip_prefix(' ').unwrap_or(raw_line);
                let old_no = pending.old_cursor;
                let new_no = pending.new_cursor;
                pending.old_cursor += 1;
                pending.new_cursor += 1;
                if pending.lines.len() < MAX_LINES_PER_HUNK {
                    pending.lines.push(MagitDiffLine {
                        kind: MagitDiffLineKind::Context,
                        old_line: Some(old_no),
                        new_line: Some(new_no),
                        text: text.to_owned(),
                    });
                }
            }
        }
    }

    flush(&mut cur_hunk, &mut out);
    out
}

/// Pure path-based surface grouping (transparent structural classification only;
/// never asserts a security or correctness verdict on its own).
pub fn classify_structural_concern(path: &str) -> MagitAuditConcern {
    let lower = path.to_ascii_lowercase();
    if lower.contains("/test")
        || lower.contains("_test.")
        || lower.ends_with("-test.ss")
        || lower.starts_with("docs/")
        || lower.ends_with(".md")
        || lower.ends_with(".org")
    {
        return MagitAuditConcern::TestsDocs;
    }
    if lower.contains("auth")
        || lower.contains("gate")
        || lower.contains("security")
        || lower.contains("policy")
        || lower.contains("governance")
    {
        return MagitAuditConcern::SecurityBoundary;
    }
    if lower.ends_with(".proto")
        || lower.contains("protocol")
        || lower.contains("c-abi")
        || lower.contains("grpc")
        || lower.contains("types.rs")
        || lower.contains("types.ss")
    {
        return MagitAuditConcern::ProtocolAbi;
    }
    if lower.contains("runtime")
        || lower.contains("async")
        || lower.contains("actor")
        || lower.contains("lock")
        || lower.contains("state")
    {
        return MagitAuditConcern::ConcurrencyState;
    }
    if lower.contains("theme")
        || lower.contains("layout")
        || lower.contains("view")
        || lower.ends_with(".css")
        || lower.contains("emacs/buffer")
    {
        return MagitAuditConcern::UiLayout;
    }
    if lower.ends_with(".toml")
        || lower.ends_with(".json")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with("justfile")
        || lower.contains("build.ss")
    {
        return MagitAuditConcern::ConfigBuild;
    }
    MagitAuditConcern::CoreLogic
}

/// Honest Multi-Source Evidence Chain Fusion (1:1 mirror of Scheme `marlin-magit-fuse-evidence-risk`):
/// Combines REAL Lambda Aitia gRPC `g2Findings`, repository formal model status,
/// and Human / LLM joint audit verdicts into cited `evidence` items and a
/// transparent `risk` + `risk_reason`. Never fabricates `High` from regex heuristics.
pub fn fuse_hunk_evidence(
    hunk: &mut MagitDiffHunk,
    aitia_report: &AitiaLiveReport,
    formal_status: MagitFormalStatus,
    formal_model_hint: Option<&str>,
    joint_records: &[MagitJointAuditRecord],
    is_conflict: bool,
) {
    hunk.formal_status = formal_status;

    // Preserve structural diffstat as the baseline observation.
    let mut evidence = vec![MagitEvidenceItem {
        source: MagitEvidenceSource::Structural,
        verdict: "diffstat".to_owned(),
        rule_or_actor: hunk.concern.label().to_owned(),
        summary: format!(
            "Structural diffstat (+{}/-{}) on {} surface",
            hunk.added,
            hunk.removed,
            hunk.concern.label()
        ),
        line: Some(hunk.new_start),
    }];

    // 1. Attach REAL Lambda Aitia gRPC G2 findings intersecting this hunk's line range.
    let range_start = hunk.new_start;
    let range_end = hunk.new_start.saturating_add(hunk.new_count.max(1));
    let mut aitia_hits = 0u32;
    let mut first_rule = String::new();

    if aitia_report.online {
        for finding in &aitia_report.findings {
            if finding.file == hunk.file_path
                && (finding.line == 0
                    || (finding.line >= range_start && finding.line <= range_end))
            {
                aitia_hits += 1;
                if first_rule.is_empty() {
                    first_rule = format!("{}/{}", finding.hallucination_class, finding.rule);
                }
                evidence.push(MagitEvidenceItem {
                    source: MagitEvidenceSource::AitiaGrpc,
                    verdict: "finding".to_owned(),
                    rule_or_actor: format!(
                        "{}/{}/{}",
                        finding.subdomain, finding.hallucination_class, finding.rule
                    ),
                    summary: if finding.excerpt.is_empty() {
                        format!("Aitia G2 {} hit on {}", finding.rule, finding.surface)
                    } else {
                        finding.excerpt.clone()
                    },
                    line: if finding.line > 0 {
                        Some(finding.line)
                    } else {
                        None
                    },
                });
            }
        }
        if aitia_hits == 0 {
            evidence.push(MagitEvidenceItem {
                source: MagitEvidenceSource::AitiaGrpc,
                verdict: aitia_report.decision.to_ascii_lowercase(),
                rule_or_actor: aitia_report.engine_owner.clone(),
                summary: format!(
                    "Aitia gRPC {}: 0 findings on this hunk",
                    aitia_report.decision
                ),
                line: None,
            });
        }
    } else {
        evidence.push(MagitEvidenceItem {
            source: MagitEvidenceSource::AitiaGrpc,
            verdict: "offline".to_owned(),
            rule_or_actor: "aitia.marlin.v1".to_owned(),
            summary: aitia_report.note.clone(),
            line: None,
        });
    }

    // 2. Attach Formal Model linkage (Quint / SysML) honestly.
    match formal_status {
        MagitFormalStatus::Verified => {
            evidence.push(MagitEvidenceItem {
                source: MagitEvidenceSource::FormalModel,
                verdict: "verified".to_owned(),
                rule_or_actor: formal_model_hint.unwrap_or("quint/sysml").to_owned(),
                summary: "Formal model evidence verified for current context digest".to_owned(),
                line: None,
            });
        }
        MagitFormalStatus::ModelPresentUnrun => {
            evidence.push(MagitEvidenceItem {
                source: MagitEvidenceSource::FormalModel,
                verdict: "pending".to_owned(),
                rule_or_actor: formal_model_hint.unwrap_or("dev/quint").to_owned(),
                summary: format!(
                    "Formal model exists ({}) — verification not yet attested for this diff",
                    formal_model_hint.unwrap_or("dev/quint")
                ),
                line: None,
            });
        }
        MagitFormalStatus::None => {}
    }

    // 3. Attach Human / LLM Joint Audit evidence records for this hunk.
    let mut latest_joint_verdict = "unreviewed";
    for rec in joint_records {
        if rec.hunk_id == hunk.hunk_id {
            latest_joint_verdict = rec.verdict.as_str();
            let src = if rec.actor == "llm" {
                MagitEvidenceSource::LlmAudit
            } else {
                MagitEvidenceSource::HumanAudit
            };
            evidence.push(MagitEvidenceItem {
                source: src,
                verdict: rec.verdict.clone(),
                rule_or_actor: rec.actor.clone(),
                summary: if rec.rationale.trim().is_empty() {
                    format!("Marked {} by {}", rec.verdict, rec.actor)
                } else {
                    rec.rationale.clone()
                },
                line: None,
            });
        }
    }

    // 4. Derive final risk strictly from cited evidence (zero blind heuristic!).
    let (risk, reason) = if is_conflict {
        (
            MagitAuditRisk::High,
            "Unresolved merge/jj conflict — requires Human + AI 3-way resolution".to_owned(),
        )
    } else if latest_joint_verdict == "flagged" || latest_joint_verdict == "disputed" {
        (
            MagitAuditRisk::High,
            format!("Flagged ({latest_joint_verdict}) in Human-AI joint audit evidence"),
        )
    } else if aitia_hits > 0 {
        (
            MagitAuditRisk::High,
            format!("Lambda Aitia G2 reported {aitia_hits} finding(s): {first_rule}"),
        )
    } else if !aitia_report.online {
        (
            MagitAuditRisk::Unassessed,
            format!(
                "Aitia offline — structural diffstat only (+{}/-{})",
                hunk.added, hunk.removed
            ),
        )
    } else if latest_joint_verdict == "audited" && formal_status == MagitFormalStatus::Verified {
        (
            MagitAuditRisk::Info,
            "Verified: Lambda Aitia ADMIT + Formal Evidence + Human-AI audited".to_owned(),
        )
    } else if latest_joint_verdict == "audited" {
        (
            MagitAuditRisk::Low,
            "Lambda Aitia ADMIT (0 findings) + Human-AI audited".to_owned(),
        )
    } else if formal_status == MagitFormalStatus::ModelPresentUnrun
        && hunk.concern != MagitAuditConcern::TestsDocs
    {
        (
            MagitAuditRisk::Medium,
            format!(
                "Aitia ADMIT (0 findings) · Formal model pending ({})",
                formal_model_hint.unwrap_or("dev/quint")
            ),
        )
    } else if hunk.concern == MagitAuditConcern::TestsDocs {
        (
            MagitAuditRisk::Info,
            format!(
                "Aitia ADMIT (0 findings) · Tests/docs (+{}/-{})",
                hunk.added, hunk.removed
            ),
        )
    } else {
        (
            MagitAuditRisk::Low,
            format!(
                "Aitia ADMIT (0 findings) · awaiting Human/LLM confirmation (+{}/-{})",
                hunk.added, hunk.removed
            ),
        )
    };

    hunk.risk = risk;
    hunk.risk_reason = reason;
    hunk.evidence = evidence;
}

/// Builds a valid unified diff patch for a single hunk (`magit-apply.el` style),
/// ready to pipe into `git apply --cached --unidiff-zero -` or `--reverse`.
pub fn build_single_hunk_patch(hunk: &MagitDiffHunk) -> String {
    let mut patch = String::new();
    patch.push_str(&format!("--- a/{}\n", hunk.file_path));
    patch.push_str(&format!("+++ b/{}\n", hunk.file_path));
    patch.push_str(&hunk.header);
    patch.push('\n');
    for line in &hunk.lines {
        let prefix = match line.kind {
            MagitDiffLineKind::Added => '+',
            MagitDiffLineKind::Removed => '-',
            MagitDiffLineKind::Context => ' ',
        };
        patch.push(prefix);
        patch.push_str(&line.text);
        patch.push('\n');
    }
    patch
}

struct PendingHunk {
    hunk_id: String,
    file_path: String,
    header: String,
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    old_cursor: u32,
    new_cursor: u32,
    added: u32,
    removed: u32,
    lines: Vec<MagitDiffLine>,
}

fn extract_diff_git_path(rest: &str) -> Option<String> {
    let (_, rhs) = rest.rsplit_once(" b/")?;
    let trimmed = rhs.trim().trim_matches('"');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn normalize_diff_header_path(rest: &str) -> Option<String> {
    let trimmed = rest.trim().trim_matches('"');
    if trimmed == "/dev/null" {
        return None;
    }
    let clean = trimmed.strip_prefix("b/").unwrap_or(trimmed);
    if clean.is_empty() {
        None
    } else {
        Some(clean.to_owned())
    }
}

fn parse_hunk_ranges(header: &str) -> (u32, u32, u32, u32) {
    let Some(inner) = header.strip_prefix("@@ ") else {
        return (1, 0, 1, 0);
    };
    let Some((ranges, _)) = inner.split_once(" @@") else {
        return (1, 0, 1, 0);
    };
    let mut parts = ranges.split_whitespace();
    let old_part = parts.next().unwrap_or("-1,0").trim_start_matches('-');
    let new_part = parts.next().unwrap_or("+1,0").trim_start_matches('+');
    let (os, oc) = parse_pair(old_part);
    let (ns, nc) = parse_pair(new_part);
    (os, oc, ns, nc)
}

fn parse_pair(s: &str) -> (u32, u32) {
    if let Some((a, b)) = s.split_once(',') {
        (a.parse().unwrap_or(1), b.parse().unwrap_or(0))
    } else {
        (s.parse().unwrap_or(1), 1)
    }
}
