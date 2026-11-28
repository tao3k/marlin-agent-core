// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::HashMap;

use crate::{
    v1::{WorkbenchTabKindProto, WorkbenchTabsResponse},
    wire::marlin::{
        gate::v1::PendingGate,
        projection::v1::GraphRecordView,
    },
};

/// Typed read-only view of a Scheme `FleetAuditCapsule` projected over
/// `ProjectionQueryService::query_project_graph`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAuditCapsuleView {
    pub record_id: String,
    pub kind: String,
    pub summary: String,
    pub active_actor_count: usize,
    pub archived_actor_count: usize,
    pub peek_count: usize,
    pub total_active_defeaters: usize,
    pub total_human_inbox_items: usize,
    pub disclosure_level: String,
    pub favored_path_collapsed: bool,
    pub authenticates_evidence: bool,
    pub authorizes_effect: bool,
}

impl FleetAuditCapsuleView {
    pub fn from_record(rec: &GraphRecordView) -> Result<Self, String> {
        let parse_usize = |k: &str| -> Result<usize, String> {
            match rec.facets.get(k) {
                None => Ok(0),
                Some(raw) => raw
                    .trim()
                    .parse::<usize>()
                    .map_err(|err| format!("invalid fleet audit facet {k:?}={raw:?}: {err}")),
            }
        };
        let parse_bool = |k: &str| -> bool {
            rec.facets
                .get(k)
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
        };
        Ok(Self {
            record_id: rec.record_id.clone(),
            kind: rec.kind.clone(),
            summary: rec.summary.clone(),
            active_actor_count: parse_usize("active_actor_count")?,
            archived_actor_count: parse_usize("archived_actor_count")?,
            peek_count: parse_usize("peek_count")?,
            total_active_defeaters: parse_usize("total_active_defeaters")?,
            total_human_inbox_items: parse_usize("total_human_inbox_items")?,
            disclosure_level: rec
                .facets
                .get("disclosure_level")
                .cloned()
                .unwrap_or_else(|| "L1".to_owned()),
            favored_path_collapsed: parse_bool("favored_path_collapsed"),
            authenticates_evidence: parse_bool("authenticates_evidence"),
            authorizes_effect: parse_bool("authorizes_effect"),
        })
    }

    /// Serializes this capsule view as an RFC 8259 JSON object without external serde deps.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"record_id\":{},\"kind\":{},\"summary\":{},\"active_actor_count\":{},\"archived_actor_count\":{},\"peek_count\":{},\"total_active_defeaters\":{},\"total_human_inbox_items\":{},\"disclosure_level\":{},\"favored_path_collapsed\":{},\"authenticates_evidence\":{},\"authorizes_effect\":{}}}",
            json_escape(&self.record_id),
            json_escape(&self.kind),
            json_escape(&self.summary),
            self.active_actor_count,
            self.archived_actor_count,
            self.peek_count,
            self.total_active_defeaters,
            self.total_human_inbox_items,
            json_escape(&self.disclosure_level),
            self.favored_path_collapsed,
            self.authenticates_evidence,
            self.authorizes_effect
        )
    }
}

/// Typed view of the live compiled-Cedar probe result projected in
/// `GovernanceAuthoritySnapshotView::last_cedar_probe_json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CedarProbeResultView {
    pub seq: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub step_up_fresh: bool,
    pub decision: String,
    pub matched_policy_ids_json: String,
    pub raw_json: String,
}

impl CedarProbeResultView {
    /// Parses a canonical `CedarProbeResult` JSON object without external `serde` dependencies.
    pub fn from_json_object(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
            return None;
        }
        let seq = extract_json_u64(trimmed, "seq")?;
        let actor = extract_json_string(trimmed, "actor")?;
        let action = extract_json_string(trimmed, "action")?;
        let resource = extract_json_string(trimmed, "resource")?;
        let step_up_fresh = extract_json_bool(trimmed, "step_up_fresh")?;
        let decision = extract_json_string(trimmed, "decision")?;
        let matched_policy_ids_json =
            extract_json_array(trimmed, "matched_policy_ids").unwrap_or_else(|| "[]".to_owned());
        Some(Self {
            seq,
            actor,
            action,
            resource,
            step_up_fresh,
            decision,
            matched_policy_ids_json,
            raw_json: trimmed.to_owned(),
        })
    }

    /// Returns the canonical RFC 8259 JSON representation of this probe result.
    pub fn to_json(&self) -> String {
        self.raw_json.clone()
    }
}

fn find_json_field_value<'a>(obj: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":");
    let pos = obj.find(&needle)?;
    Some(obj[pos + needle.len()..].trim_start())
}

fn extract_json_u64(obj: &str, key: &str) -> Option<u64> {
    let tail = find_json_field_value(obj, key)?;
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u64>().ok()
}

fn extract_json_bool(obj: &str, key: &str) -> Option<bool> {
    let tail = find_json_field_value(obj, key)?;
    if tail.starts_with("true") {
        Some(true)
    } else if tail.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_json_string(obj: &str, key: &str) -> Option<String> {
    let tail = find_json_field_value(obj, key)?;
    let mut chars = tail.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            match ch {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn extract_json_array(obj: &str, key: &str) -> Option<String> {
    let tail = find_json_field_value(obj, key)?;
    if !tail.starts_with('[') {
        return None;
    }
    let mut in_str = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for (idx, ch) in tail.char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(tail[..=idx].to_owned());
                }
            }
            _ => {}
        }
    }
    None
}

/// Typed read-only view of a Scheme `GovernanceAuthoritySnapshot` projected over
/// `ProjectionQueryService::query_project_graph` (`kind == "scheme.governance.authority_snapshot"`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceAuthoritySnapshotView {
    pub record_id: String,
    pub summary: String,
    pub admitted: bool,
    pub revocation_epoch: u64,
    pub policy_revision: u64,
    pub breaker_tripped: bool,
    pub breaker_trip_epoch: u64,
    pub breaker_budget_used: u64,
    pub breaker_budget_limit: u64,
    pub breaker_active_leases: u64,
    pub rearm_gate_id: String,
    pub assessment_digest: String,
    pub taint_proof_digest: String,
    pub fixpoint_proof_digest: String,
    pub temporal_proof_digest: String,
    pub invalidation_digest: String,
    pub rejection_reasons_json: String,
    pub fenced_actors_json: String,
    pub blocked_tools_json: String,
    pub retracted_conclusions_json: String,
    pub cedar_policies_json: String,
    pub fact_taint_edges_json: String,
    pub fact_taint_sources_json: String,
    pub fact_taint_sinks_json: String,
    pub fact_taint_sanitizers_json: String,
    pub last_cedar_probe_json: Option<String>,
    pub last_event_note: String,
}

impl GovernanceAuthoritySnapshotView {
    pub fn from_record(rec: &GraphRecordView) -> Result<Self, String> {
        let parse_u64 = |k: &str| -> Result<u64, String> {
            match rec.facets.get(k) {
                None => Ok(0),
                Some(raw) => raw
                    .trim()
                    .parse::<u64>()
                    .map_err(|err| format!("invalid governance numeric facet {k:?}={raw:?}: {err}")),
            }
        };
        let parse_bool = |k: &str| -> bool {
            rec.facets
                .get(k)
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
        };
        let get_str = |k: &str| -> String { rec.facets.get(k).cloned().unwrap_or_default() };
        let get_json_array = |k: &str| -> String {
            rec.facets
                .get(k)
                .map(|v| v.trim())
                .filter(|v| v.starts_with('[') && v.ends_with(']'))
                .map_or_else(|| "[]".to_owned(), str::to_owned)
        };
        let get_json_object = |k: &str| -> Option<String> {
            rec.facets
                .get(k)
                .map(|v| v.trim())
                .filter(|v| v.starts_with('{') && v.ends_with('}'))
                .map(str::to_owned)
        };
        Ok(Self {
            record_id: rec.record_id.clone(),
            summary: rec.summary.clone(),
            admitted: parse_bool("admitted"),
            revocation_epoch: parse_u64("revocation_epoch")?,
            policy_revision: parse_u64("policy_revision")?,
            breaker_tripped: parse_bool("breaker_tripped"),
            breaker_trip_epoch: parse_u64("breaker_trip_epoch")?,
            breaker_budget_used: parse_u64("breaker_budget_used")?,
            breaker_budget_limit: parse_u64("breaker_budget_limit")?,
            breaker_active_leases: parse_u64("breaker_active_leases")?,
            rearm_gate_id: get_str("rearm_gate_id"),
            assessment_digest: get_str("assessment_digest"),
            taint_proof_digest: get_str("taint_proof_digest"),
            fixpoint_proof_digest: get_str("fixpoint_proof_digest"),
            temporal_proof_digest: get_str("temporal_proof_digest"),
            invalidation_digest: get_str("invalidation_digest"),
            rejection_reasons_json: get_json_array("rejection_reasons_json"),
            fenced_actors_json: get_json_array("fenced_actors_json"),
            blocked_tools_json: get_json_array("blocked_tools_json"),
            retracted_conclusions_json: get_json_array("retracted_conclusions_json"),
            cedar_policies_json: get_json_array("cedar_policies_json"),
            fact_taint_edges_json: get_json_array("fact_taint_edges_json"),
            fact_taint_sources_json: get_json_array("fact_taint_sources_json"),
            fact_taint_sinks_json: get_json_array("fact_taint_sinks_json"),
            fact_taint_sanitizers_json: get_json_array("fact_taint_sanitizers_json"),
            last_cedar_probe_json: get_json_object("last_cedar_probe_json"),
            last_event_note: get_str("last_event_note"),
        })
    }

    /// Parses `last_cedar_probe_json` into a typed `CedarProbeResultView` when present.
    pub fn last_cedar_probe(&self) -> Option<CedarProbeResultView> {
        self.last_cedar_probe_json
            .as_deref()
            .and_then(CedarProbeResultView::from_json_object)
    }

    /// Serializes this governance snapshot as an RFC 8259 JSON object without external serde deps.
    pub fn to_json(&self) -> String {
        let probe_json = self.last_cedar_probe_json.as_deref().unwrap_or("null");
        format!(
            "{{\"record_id\":{},\"summary\":{},\"admitted\":{},\"revocation_epoch\":{},\"policy_revision\":{},\"breaker_tripped\":{},\"breaker_trip_epoch\":{},\"breaker_budget_used\":{},\"breaker_budget_limit\":{},\"breaker_active_leases\":{},\"rearm_gate_id\":{},\"assessment_digest\":{},\"taint_proof_digest\":{},\"fixpoint_proof_digest\":{},\"temporal_proof_digest\":{},\"invalidation_digest\":{},\"rejection_reasons\":{},\"fenced_actors\":{},\"blocked_tools\":{},\"retracted_conclusions\":{},\"cedar_policies\":{},\"fact_taint_edges\":{},\"fact_taint_sources\":{},\"fact_taint_sinks\":{},\"fact_taint_sanitizers\":{},\"last_cedar_probe\":{probe_json},\"last_event_note\":{}}}",
            json_escape(&self.record_id),
            json_escape(&self.summary),
            self.admitted,
            self.revocation_epoch,
            self.policy_revision,
            self.breaker_tripped,
            self.breaker_trip_epoch,
            self.breaker_budget_used,
            self.breaker_budget_limit,
            self.breaker_active_leases,
            json_escape(&self.rearm_gate_id),
            json_escape(&self.assessment_digest),
            json_escape(&self.taint_proof_digest),
            json_escape(&self.fixpoint_proof_digest),
            json_escape(&self.temporal_proof_digest),
            json_escape(&self.invalidation_digest),
            self.rejection_reasons_json,
            self.fenced_actors_json,
            self.blocked_tools_json,
            self.retracted_conclusions_json,
            self.cedar_policies_json,
            self.fact_taint_edges_json,
            self.fact_taint_sources_json,
            self.fact_taint_sinks_json,
            self.fact_taint_sanitizers_json,
            json_escape(&self.last_event_note),
        )
    }
}

/// Unified operator snapshot combining live `WorkbenchTabs`, read-only `FleetAuditCapsule`,
/// `GovernanceAuthoritySnapshot`, and pending one-shot `ApprovalGateService` gates from a
/// single HTTP/2 round-trip batch.
#[derive(Clone, Debug)]
pub struct HostStatusSnapshot {
    pub tabs: WorkbenchTabsResponse,
    pub audit: Option<FleetAuditCapsuleView>,
    pub governance: Option<GovernanceAuthoritySnapshotView>,
    pub gates: Vec<PendingGate>,
}

impl HostStatusSnapshot {
    /// Serializes this snapshot as a deterministic RFC 8259 JSON object.
    pub fn to_json(&self) -> String {
        let tabs_json: Vec<String> = self
            .tabs
            .tabs
            .iter()
            .map(|t| {
                format!(
                    "{{\"kind\":{},\"id\":{},\"fencing_epoch\":{},\"status_label\":{}}}",
                    json_escape(workbench_tab_kind_label(t.kind)),
                    json_escape(&t.id),
                    t.fencing_epoch,
                    json_escape(&t.status_label)
                )
            })
            .collect();
        let audit_json = self
            .audit
            .as_ref()
            .map_or_else(|| "null".to_owned(), FleetAuditCapsuleView::to_json);
        let gov_json = self.governance.as_ref().map_or_else(
            || "null".to_owned(),
            GovernanceAuthoritySnapshotView::to_json,
        );
        let gates_json: Vec<String> = self
            .gates
            .iter()
            .map(|g| {
                format!(
                    "{{\"gate_id\":{},\"tool_or_node_name\":{}}}",
                    json_escape(&g.gate_id),
                    json_escape(&g.tool_or_node_name)
                )
            })
            .collect();
        format!(
            "{{\"tabs\":[{}],\"audit\":{},\"governance\":{},\"gates\":[{}]}}",
            tabs_json.join(","),
            audit_json,
            gov_json,
            gates_json.join(",")
        )
    }
}

/// Typed incremental runtime fact-graph mutations accepted by `marlin-scheme-host` /
/// `marlin-scheme-host-aot` (`scheme:governance-*` declarative commands).
///
/// Security invariant: re-arm is deliberately excluded; step-up re-arm is strictly
/// routed through `ApprovalGateService::submit_graph_human_decision`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceFactAction {
    EvalClean,
    EvalIncident,
    Delegate {
        parent: String,
        child: String,
    },
    Revoke {
        actor: String,
    },
    TaintEdge {
        src: String,
        dst: String,
        role: Option<String>,
    },
    Sanitize {
        node: String,
    },
    ContextFlow {
        entry: String,
        src_scope: String,
        target: String,
        dst_scope: String,
    },
    Declassify {
        entry: String,
    },
    Retract {
        subject: String,
    },
    RegisterDerivation {
        derivation_id: String,
        policy_id: String,
        subject_premises: Vec<String>,
        parent_derivations: Vec<String>,
    },
    CheckCedar {
        actor: String,
        action: String,
        resource: String,
        step_up_fresh: bool,
    },
}

impl GovernanceFactAction {
    /// Projects this typed action into the `(action_sym, arg)` wire pair expected by
    /// `MarlinSchemeHostClient::dispatch_scheme_command`.
    pub fn to_wire_command(&self) -> (&'static str, Option<String>) {
        match self {
            Self::EvalClean => ("governance-eval-clean", None),
            Self::EvalIncident => ("governance-eval-incident", None),
            Self::Delegate { parent, child } => {
                ("governance-delegate", Some(format!("{parent} {child}")))
            }
            Self::Revoke { actor } => ("governance-revoke", Some(actor.clone())),
            Self::TaintEdge { src, dst, role } => {
                let arg = match role.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
                    Some(r) => format!("{src} {dst} {r}"),
                    None => format!("{src} {dst}"),
                };
                ("governance-taint-edge", Some(arg))
            }
            Self::Sanitize { node } => ("governance-sanitize", Some(node.clone())),
            Self::ContextFlow {
                entry,
                src_scope,
                target,
                dst_scope,
            } => (
                "governance-context-flow",
                Some(format!("{entry} {src_scope} {target} {dst_scope}")),
            ),
            Self::Declassify { entry } => ("governance-declassify", Some(entry.clone())),
            Self::Retract { subject } => ("governance-retract", Some(subject.clone())),
            Self::RegisterDerivation {
                derivation_id,
                policy_id,
                subject_premises,
                parent_derivations,
            } => {
                let premises = subject_premises.join(",");
                let parents = if parent_derivations.is_empty() {
                    "none".to_owned()
                } else {
                    parent_derivations.join(",")
                };
                (
                    "governance-register-derivation",
                    Some(format!("{derivation_id} {policy_id} {premises} {parents}")),
                )
            }
            Self::CheckCedar {
                actor,
                action,
                resource,
                step_up_fresh,
            } => (
                "governance-check-cedar",
                Some(format!("{actor} {action} {resource} {step_up_fresh}")),
            ),
        }
    }
}

/// Typed fleet / task steering commands routed through `UiWorkbenchService::DispatchUiEvent`
/// (`scheme:task-*` namespace). Covers all 15 Scheme fleet steering actions while refusing
/// privileged human approval or breaker re-arm commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetTaskAction {
    /// Progressive Disclosure depth L0–L3 (`level` clamped/validated to `0..=3`).
    Drill {
        level: u8,
    },
    ProposeBranch {
        label: Option<String>,
    },
    DetourBlocker {
        note: Option<String>,
    },
    DetourSidequest {
        note: Option<String>,
    },
    DetourExitReturn,
    DetourExitAbsorb,
    DetourExitAbort,
    DetourExitSibling,
    Peek {
        actor_id: Option<String>,
    },
    Reopen {
        actor_id: Option<String>,
    },
    FleetStep,
    SandboxVerify,
}

impl FleetTaskAction {
    /// Projects this typed fleet task command into the `(action_sym, arg)` wire pair expected
    /// by `MarlinSchemeHostClient::dispatch_scheme_command`.
    pub fn to_wire_command(&self) -> (&'static str, Option<String>) {
        let trim_opt = |opt: &Option<String>| {
            opt.as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        match self {
            Self::Drill { level: 0 } => ("task-drill-l0", None),
            Self::Drill { level: 1 } => ("task-drill-l1", None),
            Self::Drill { level: 2 } => ("task-drill-l2", None),
            Self::Drill { .. } => ("task-drill-l3", None),
            Self::ProposeBranch { label } => ("task-propose-branch", trim_opt(label)),
            Self::DetourBlocker { note } => ("task-detour-blocker", trim_opt(note)),
            Self::DetourSidequest { note } => ("task-detour-sidequest", trim_opt(note)),
            Self::DetourExitReturn => ("task-detour-exit-return", None),
            Self::DetourExitAbsorb => ("task-detour-exit-absorb", None),
            Self::DetourExitAbort => ("task-detour-exit-abort", None),
            Self::DetourExitSibling => ("task-detour-exit-sibling", None),
            Self::Peek { actor_id } => ("task-peek", trim_opt(actor_id)),
            Self::Reopen { actor_id } => ("task-reopen", trim_opt(actor_id)),
            Self::FleetStep => ("task-fleet-step", None),
            Self::SandboxVerify => ("task-sandbox-verify", None),
        }
    }
}

pub fn workbench_tab_kind_label(kind: i32) -> &'static str {
    match WorkbenchTabKindProto::try_from(kind) {
        Ok(WorkbenchTabKindProto::PinnedIndex) => "PinnedIndex",
        Ok(WorkbenchTabKindProto::ActiveTask) => "ActiveTask",
        Ok(WorkbenchTabKindProto::ReadOnlyPeek) => "ReadOnlyPeek",
        _ => "Unspecified",
    }
}

/// Quotes and escapes a Rust `&str` as an RFC 8259 JSON string literal.
pub fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Formats a `GraphRecordView` as a deterministic RFC 8259 JSON object.
pub fn graph_record_to_json(rec: &GraphRecordView) -> String {
    let mut pairs: Vec<(&String, &String)> = rec.facets.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    let facets_json: Vec<String> = pairs
        .into_iter()
        .map(|(k, v)| format!("{}:{}", json_escape(k), json_escape(v)))
        .collect();
    let source_path = rec
        .source_path
        .as_deref()
        .map_or_else(|| "null".to_owned(), json_escape);
    format!(
        "{{\"record_id\":{},\"kind\":{},\"title\":{},\"summary\":{},\"source_path\":{source_path},\"facets\":{{{}}}}}",
        json_escape(&rec.record_id),
        json_escape(&rec.kind),
        json_escape(&rec.title),
        json_escape(&rec.summary),
        facets_json.join(",")
    )
}

/// Formats a `GraphRecordView` facets map deterministically for CLI output.
pub fn format_facets_sorted(facets: &HashMap<String, String>) -> String {
    let mut pairs: Vec<(&String, &String)> = facets.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ")
}
