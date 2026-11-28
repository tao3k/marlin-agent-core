// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::HashMap;

use marlin_agent_app_server::{
    scheme_host_client::{
        CedarProbeResultView, FleetAuditCapsuleView, FleetTaskAction,
        GovernanceAuthoritySnapshotView, GovernanceFactAction, HostStatusSnapshot,
        graph_record_to_json, json_escape, parse_bearer_header,
    },
    v1::WorkbenchTabsResponse,
    wire::marlin::projection::v1::GraphRecordView,
};

#[test]
fn json_escape_handles_quotes_backslashes_and_control_chars() {
    assert_eq!(json_escape("hello"), "\"hello\"");
    assert_eq!(
        json_escape("a\"b\\c\nd\te\x01"),
        "\"a\\\"b\\\\c\\nd\\te\\u0001\""
    );
}

#[test]
fn graph_record_to_json_orders_facets_deterministically() {
    let mut facets = HashMap::new();
    facets.insert("z_key".to_owned(), "2".to_owned());
    facets.insert("a_key".to_owned(), "1".to_owned());
    let rec = GraphRecordView {
        record_id: "scheme:fleet:audit-capsule".to_owned(),
        surface: 0,
        kind: "scheme.fleet.audit_capsule".to_owned(),
        title: "Audit".to_owned(),
        summary: "ok".to_owned(),
        source_path: Some("scheme/graph-engine/fleet-controller.ss".to_owned()),
        start_line: None,
        end_line: None,
        facets,
    };
    let raw = graph_record_to_json(&rec);
    assert!(raw.contains("\"facets\":{\"a_key\":\"1\",\"z_key\":\"2\"}"));
    assert!(raw.contains("\"source_path\":\"scheme/graph-engine/fleet-controller.ss\""));
}

#[test]
fn host_status_snapshot_serializes_valid_structure() {
    let mut gov_facets = HashMap::new();
    gov_facets.insert("admitted".to_owned(), "false".to_owned());
    gov_facets.insert("revocation_epoch".to_owned(), "2".to_owned());
    gov_facets.insert("policy_revision".to_owned(), "3".to_owned());
    gov_facets.insert("breaker_tripped".to_owned(), "true".to_owned());
    gov_facets.insert("breaker_trip_epoch".to_owned(), "1".to_owned());
    gov_facets.insert("breaker_budget_used".to_owned(), "4".to_owned());
    gov_facets.insert("breaker_budget_limit".to_owned(), "64".to_owned());
    gov_facets.insert("rearm_gate_id".to_owned(), "gov-rearm-epoch-1".to_owned());
    gov_facets.insert(
        "fenced_actors_json".to_owned(),
        "[\"agent-rogue\"]".to_owned(),
    );
    gov_facets.insert(
        "fact_taint_edges_json".to_owned(),
        "[{\"src\":\"in\",\"dst\":\"pipe\"}]".to_owned(),
    );
    gov_facets.insert("fact_taint_sources_json".to_owned(), "[\"in\"]".to_owned());
    gov_facets.insert("fact_taint_sinks_json".to_owned(), "[\"sink\"]".to_owned());
    gov_facets.insert(
        "fact_taint_sanitizers_json".to_owned(),
        "[\"pipe\"]".to_owned(),
    );
    gov_facets.insert(
        "last_cedar_probe_json".to_owned(),
        "{\"seq\":1,\"actor\":\"agent-rogue\",\"action\":\"run_tool\",\"resource\":\"workspace\",\"step_up_fresh\":false,\"decision\":\"deny\",\"matched_policy_ids\":[\"marlin.ascent.forbid.fenced.agent-rogue\"]}".to_owned(),
    );
    let gov_rec = GraphRecordView {
        record_id: "scheme:governance:authority-snapshot".to_owned(),
        surface: 0,
        kind: "scheme.governance.authority_snapshot".to_owned(),
        title: "Governance".to_owned(),
        summary: "admitted=false".to_owned(),
        source_path: Some("scheme/policy/governance.ss".to_owned()),
        start_line: None,
        end_line: None,
        facets: gov_facets,
    };
    let gov = GovernanceAuthoritySnapshotView::from_record(&gov_rec)
        .expect("valid governance snapshot facets must parse");
    assert!(!gov.admitted);

    // Fail-closed negative control: malformed numeric governance facet MUST NOT collapse to 0.
    let mut corrupt_gov_rec = gov_rec.clone();
    corrupt_gov_rec
        .facets
        .insert("revocation_epoch".to_owned(), "tampered-not-u64".to_owned());
    assert!(
        GovernanceAuthoritySnapshotView::from_record(&corrupt_gov_rec).is_err(),
        "malformed revocation_epoch facet must fail closed instead of collapsing to 0"
    );
    assert!(gov.breaker_tripped);
    assert_eq!(gov.rearm_gate_id, "gov-rearm-epoch-1");
    assert_eq!(gov.fact_taint_sources_json, "[\"in\"]");
    assert_eq!(gov.fact_taint_sanitizers_json, "[\"pipe\"]");
    assert!(gov.last_cedar_probe_json.is_some());

    let snap = HostStatusSnapshot {
        tabs: WorkbenchTabsResponse {
            focused_task_id: None,
            tabs: Vec::new(),
        },
        audit: Some(FleetAuditCapsuleView {
            record_id: "scheme:fleet:audit-capsule".to_owned(),
            kind: "scheme.fleet.audit_capsule".to_owned(),
            summary: "active=1".to_owned(),
            active_actor_count: 1,
            archived_actor_count: 1,
            peek_count: 1,
            total_active_defeaters: 0,
            total_human_inbox_items: 1,
            disclosure_level: "L3".to_owned(),
            favored_path_collapsed: false,
            authenticates_evidence: false,
            authorizes_effect: false,
        }),
        governance: Some(gov),
        gates: Vec::new(),
    };
    let raw = snap.to_json();
    assert!(raw.contains("\"tabs\":[]"));
    assert!(raw.contains("\"active_actor_count\":1"));
    assert!(raw.contains("\"authorizes_effect\":false"));
    assert!(raw.contains("\"breaker_tripped\":true"));
    assert!(raw.contains("\"fenced_actors\":[\"agent-rogue\"]"));
    assert!(raw.contains("\"fact_taint_edges\":[{\"src\":\"in\",\"dst\":\"pipe\"}]"));
    assert!(raw.contains("\"fact_taint_sources\":[\"in\"]"));
    assert!(raw.contains("\"fact_taint_sinks\":[\"sink\"]"));
    assert!(raw.contains("\"fact_taint_sanitizers\":[\"pipe\"]"));
    assert!(raw.contains("\"last_cedar_probe\":{\"seq\":1,\"actor\":\"agent-rogue\""));
    assert!(raw.contains("\"rearm_gate_id\":\"gov-rearm-epoch-1\""));
    assert!(raw.contains("\"gates\":[]"));
}

#[test]
fn governance_fact_action_formats_all_wire_commands() {
    assert_eq!(
        GovernanceFactAction::EvalClean.to_wire_command(),
        ("governance-eval-clean", None)
    );
    assert_eq!(
        GovernanceFactAction::EvalIncident.to_wire_command(),
        ("governance-eval-incident", None)
    );
    assert_eq!(
        GovernanceFactAction::Delegate {
            parent: "ops-root".to_owned(),
            child: "agent-lead".to_owned(),
        }
        .to_wire_command(),
        (
            "governance-delegate",
            Some("ops-root agent-lead".to_owned())
        )
    );
    assert_eq!(
        GovernanceFactAction::Revoke {
            actor: "ops-root".to_owned(),
        }
        .to_wire_command(),
        ("governance-revoke", Some("ops-root".to_owned()))
    );
    assert_eq!(
        GovernanceFactAction::TaintEdge {
            src: "in".to_owned(),
            dst: "pipe".to_owned(),
            role: Some("source".to_owned()),
        }
        .to_wire_command(),
        ("governance-taint-edge", Some("in pipe source".to_owned()))
    );
    assert_eq!(
        GovernanceFactAction::TaintEdge {
            src: "a".to_owned(),
            dst: "b".to_owned(),
            role: None,
        }
        .to_wire_command(),
        ("governance-taint-edge", Some("a b".to_owned()))
    );
    assert_eq!(
        GovernanceFactAction::Sanitize {
            node: "pipe".to_owned(),
        }
        .to_wire_command(),
        ("governance-sanitize", Some("pipe".to_owned()))
    );
    assert_eq!(
        GovernanceFactAction::ContextFlow {
            entry: "e1".to_owned(),
            src_scope: "secret".to_owned(),
            target: "pub".to_owned(),
            dst_scope: "public".to_owned(),
        }
        .to_wire_command(),
        (
            "governance-context-flow",
            Some("e1 secret pub public".to_owned())
        )
    );
    assert_eq!(
        GovernanceFactAction::Declassify {
            entry: "e1".to_owned(),
        }
        .to_wire_command(),
        ("governance-declassify", Some("e1".to_owned()))
    );
    assert_eq!(
        GovernanceFactAction::Retract {
            subject: "claim-1".to_owned(),
        }
        .to_wire_command(),
        ("governance-retract", Some("claim-1".to_owned()))
    );
    assert_eq!(
        GovernanceFactAction::RegisterDerivation {
            derivation_id: "D-TOKEN".to_owned(),
            policy_id: "pol.cert".to_owned(),
            subject_premises: vec!["token-cert-v1".to_owned()],
            parent_derivations: Vec::new(),
        }
        .to_wire_command(),
        (
            "governance-register-derivation",
            Some("D-TOKEN pol.cert token-cert-v1 none".to_owned())
        )
    );
    assert_eq!(
        GovernanceFactAction::RegisterDerivation {
            derivation_id: "D-PLAN".to_owned(),
            policy_id: "pol.plan".to_owned(),
            subject_premises: vec!["workspace-ok".to_owned(), "scope-ok".to_owned()],
            parent_derivations: vec!["D-TOKEN".to_owned()],
        }
        .to_wire_command(),
        (
            "governance-register-derivation",
            Some("D-PLAN pol.plan workspace-ok,scope-ok D-TOKEN".to_owned())
        )
    );
    assert_eq!(
        GovernanceFactAction::CheckCedar {
            actor: "scheme-sandbox-worker".to_owned(),
            action: "run_tool".to_owned(),
            resource: "workspace".to_owned(),
            step_up_fresh: false,
        }
        .to_wire_command(),
        (
            "governance-check-cedar",
            Some("scheme-sandbox-worker run_tool workspace false".to_owned())
        )
    );
    assert_eq!(
        GovernanceFactAction::CheckCedar {
            actor: "alice".to_owned(),
            action: "rearm_breaker".to_owned(),
            resource: "workspace".to_owned(),
            step_up_fresh: true,
        }
        .to_wire_command(),
        (
            "governance-check-cedar",
            Some("alice rearm_breaker workspace true".to_owned())
        )
    );
}

#[test]
fn parse_bearer_header_marks_authorization_sensitive_and_redacts_debug() {
    let header = parse_bearer_header(Some("  top-secret-token-xyz  "))
        .expect("valid bearer")
        .expect("some header");
    assert!(header.is_sensitive());
    let dbg = format!("{header:?}");
    assert!(
        !dbg.contains("top-secret-token-xyz"),
        "sensitive bearer metadata must never leak plaintext in Debug output: {dbg}"
    );
    assert!(parse_bearer_header(Some("   ")).unwrap().is_none());
    assert!(parse_bearer_header(None).unwrap().is_none());
}

#[test]
fn fleet_task_action_formats_all_15_wire_commands() {
    assert_eq!(
        FleetTaskAction::Drill { level: 0 }.to_wire_command(),
        ("task-drill-l0", None)
    );
    assert_eq!(
        FleetTaskAction::Drill { level: 1 }.to_wire_command(),
        ("task-drill-l1", None)
    );
    assert_eq!(
        FleetTaskAction::Drill { level: 2 }.to_wire_command(),
        ("task-drill-l2", None)
    );
    assert_eq!(
        FleetTaskAction::Drill { level: 3 }.to_wire_command(),
        ("task-drill-l3", None)
    );
    assert_eq!(
        FleetTaskAction::ProposeBranch {
            label: Some("B1:Verify".to_owned()),
        }
        .to_wire_command(),
        ("task-propose-branch", Some("B1:Verify".to_owned()))
    );
    assert_eq!(
        FleetTaskAction::DetourBlocker {
            note: Some("blk-1".to_owned()),
        }
        .to_wire_command(),
        ("task-detour-blocker", Some("blk-1".to_owned()))
    );
    assert_eq!(
        FleetTaskAction::DetourSidequest { note: None }.to_wire_command(),
        ("task-detour-sidequest", None)
    );
    assert_eq!(
        FleetTaskAction::DetourExitReturn.to_wire_command(),
        ("task-detour-exit-return", None)
    );
    assert_eq!(
        FleetTaskAction::DetourExitAbsorb.to_wire_command(),
        ("task-detour-exit-absorb", None)
    );
    assert_eq!(
        FleetTaskAction::DetourExitAbort.to_wire_command(),
        ("task-detour-exit-abort", None)
    );
    assert_eq!(
        FleetTaskAction::DetourExitSibling.to_wire_command(),
        ("task-detour-exit-sibling", None)
    );
    assert_eq!(
        FleetTaskAction::Peek {
            actor_id: Some("actor-2".to_owned()),
        }
        .to_wire_command(),
        ("task-peek", Some("actor-2".to_owned()))
    );
    assert_eq!(
        FleetTaskAction::Reopen {
            actor_id: Some("actor-2".to_owned()),
        }
        .to_wire_command(),
        ("task-reopen", Some("actor-2".to_owned()))
    );
    assert_eq!(
        FleetTaskAction::FleetStep.to_wire_command(),
        ("task-fleet-step", None)
    );
    assert_eq!(
        FleetTaskAction::SandboxVerify.to_wire_command(),
        ("task-sandbox-verify", None)
    );
}

#[test]
fn cedar_probe_result_view_parses_canonical_json_without_serde() {
    let raw = r#"{"seq":7,"actor":"alice","action":"rearm_breaker","resource":"webhook-out","step_up_fresh":true,"decision":"deny","matched_policy_ids":["marlin.ascent.forbid.scope.external-unless-declassified"]}"#;
    let view = CedarProbeResultView::from_json_object(raw).expect("parse cedar probe");
    assert_eq!(view.seq, 7);
    assert_eq!(view.actor, "alice");
    assert_eq!(view.action, "rearm_breaker");
    assert_eq!(view.resource, "webhook-out");
    assert!(view.step_up_fresh);
    assert_eq!(view.decision, "deny");
    assert_eq!(
        view.matched_policy_ids_json,
        r#"["marlin.ascent.forbid.scope.external-unless-declassified"]"#
    );
    assert_eq!(view.to_json(), raw);
    assert!(CedarProbeResultView::from_json_object("not-json").is_none());
}
