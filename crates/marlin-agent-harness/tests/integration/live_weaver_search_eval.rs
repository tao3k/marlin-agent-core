// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Headless Live Weaver Evaluation Suite (`marlin-agent-harness`).
//!
//! Scenarios, read-only Cedar governance policies, ground-truth checkpoints,
//! and Bi-Temporal Causality trajectory verification are declaratively owned
//! by Gerbil Scheme (`scheme/agent-harness/`). Rust executes generically with
//! zero TUI coupling.
//!
//! Offline contract check (no network, no API keys):
//!   `cargo test --offline -p marlin-agent-harness --features live-weaver --test live_weaver_search_eval`
//!
//! Live Weaver evaluation (cost-controlled `deepseek-flash` default; load `.env` first):
//!   `cargo test --offline -p marlin-agent-harness --features live-weaver --test live_weaver_search_eval live_weaver_natural_language_search_eval_suite -- --ignored --nocapture`

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use marlin_agent_harness::live_weaver::{
    CompiledHarnessGovernanceGate, HarnessSchemeBridge, LiveEvalBudget, LiveWeaverEvalSession,
    LiveWeaverTomlConfig, QueryTierKind, TrajectoryEvent, TrajectoryEventKind, classify_tool_call,
    parse_search_receipt, render_live_eval_markdown_report, score_ground_truth,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repo root")
}

#[test]
fn live_weaver_harness_scheme_suite_governance_and_temporal_causality_contract() {
    let root = repo_root();
    let bridge = HarnessSchemeBridge::from_workspace(&root).expect("locate Marlin scheme/");

    // 1. Project declarative POO Testing Suites directly from Scheme (`test-case-one.ss` & `test-case-two.ss`).
    let suite = bridge
        .project_harness_suite()
        .expect("project Scheme SearchTestingSuiteValue");
    assert_eq!(suite.suite_id, "marlin-search-read-only-eval");
    assert_eq!(suite.scenarios.len(), 4);
    assert!(!suite.scenarios[0].enable_reflection);
    assert!(
        suite
            .scenarios
            .iter()
            .all(|s| s.max_rounds < 5 && s.target_max_rounds < 5 && !s.message.is_empty()),
        "all Test Case One scenarios must enforce < 5 max_rounds and pure English messages"
    );

    let complex_suite = bridge
        .project_harness_suite_named(Some("complex"))
        .expect("project Scheme ComplexCodingTestingSuiteValue");
    assert_eq!(
        complex_suite.suite_id,
        "marlin-complex-coding-read-only-eval"
    );
    assert_eq!(complex_suite.scenarios.len(), 4);
    assert!(
        complex_suite.scenarios.iter().all(|s| {
            s.enable_reflection
                && !s.governance.write_allowed
                && s.max_rounds < 5
                && s.target_max_rounds < 5
                && !s.message.is_empty()
        }),
        "all Test Case Two scenarios must enforce read-only governance, < 5 max_rounds, and reflection"
    );
    assert!(
        LiveEvalBudget::default().max_rounds < 5,
        "default LiveEvalBudget must strictly stay under 5 rounds"
    );

    // 2. Verify fail-closed Cedar read-only Governance denies every write & bash tool call,
    //    while permitting read-only `workspace_search`.
    let first = &suite.scenarios[0];
    assert_eq!(first.id, "model_gateway_implementations");
    assert!(!first.governance.write_allowed);
    let gate =
        CompiledHarnessGovernanceGate::compile(first.governance.clone()).expect("compile Cedar");

    let permit_search =
        gate.evaluate_tool_call(&first.id, "call-1", "workspace_search", Some("search"));
    assert!(permit_search.permitted, "workspace_search must be permitted");
    assert_eq!(permit_search.cedar_decision, "Allow");
    assert!(!permit_search.is_write);

    let permit_structural =
        gate.evaluate_tool_call(&first.id, "call-1b", "structural_query", Some("search"));
    assert!(
        permit_structural.permitted,
        "structural_query must be permitted"
    );
    assert_eq!(permit_structural.cedar_decision, "Allow");

    let permit_constraint =
        gate.evaluate_tool_call(&first.id, "call-1c", "constraint_solve", Some("search"));
    assert!(
        permit_constraint.permitted,
        "constraint_solve must be permitted"
    );
    assert_eq!(permit_constraint.cedar_decision, "Allow");

    let deny_bash = gate.evaluate_tool_call(&first.id, "call-2", "bash", Some("shell"));
    assert!(!deny_bash.permitted, "bash MUST be denied in read-only search eval");
    assert!(deny_bash.is_write);
    assert_eq!(deny_bash.cedar_decision, "Forbid");

    let find_hint = marlin_agent_harness::live_weaver::format_actionable_denial_message(
        &deny_bash.reason,
        "bash",
        "find . -maxdepth 2 -type d",
    );
    assert!(find_hint.contains("suggested_replacement:"), "{find_hint}");
    assert!(find_hint.contains("(file-outline "), "{find_hint}");

    let cat_hint = marlin_agent_harness::live_weaver::format_actionable_denial_message(
        &deny_bash.reason,
        "bash",
        "cat Cargo.toml | head -60",
    );
    assert!(
        cat_hint.contains("(read-file \"Cargo.toml\" start: 1 end: 120)"),
        "{cat_hint}"
    );

    let deny_write = gate.evaluate_tool_call(&first.id, "call-3", "write_file", Some("write"));
    assert!(!deny_write.permitted, "write_file MUST be denied in read-only search eval");
    assert!(deny_write.is_write);
    assert_eq!(deny_write.cedar_decision, "Forbid");

    // 3. Verify Bi-Temporal Causality trajectory proof in Scheme:
    //    (a) Clean read-only trajectory (with a Cedar-denied bash attempt) -> ADMITTED
    let clean_events = vec![
        TrajectoryEvent {
            event_id: "ev-1".to_owned(),
            kind: TrajectoryEventKind::PromptSubmitted,
            logical_pos: 1,
            parent_ids: Vec::new(),
            valid_pos: 1,
            knowledge_pos: 1,
            detail: "prompt".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-2".to_owned(),
            kind: TrajectoryEventKind::ModelTurnCompleted,
            logical_pos: 2,
            parent_ids: vec!["ev-1".to_owned()],
            valid_pos: 2,
            knowledge_pos: 2,
            detail: "turn 1".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-3".to_owned(),
            kind: TrajectoryEventKind::GovernanceDenied,
            logical_pos: 3,
            parent_ids: vec!["ev-2".to_owned()],
            valid_pos: 3,
            knowledge_pos: 3,
            detail: "bash denied".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-4".to_owned(),
            kind: TrajectoryEventKind::ToolDenied,
            logical_pos: 4,
            parent_ids: vec!["ev-3".to_owned()],
            valid_pos: 4,
            knowledge_pos: 4,
            detail: "tool denied".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-5".to_owned(),
            kind: TrajectoryEventKind::GovernancePermitted,
            logical_pos: 5,
            parent_ids: vec!["ev-2".to_owned()],
            valid_pos: 5,
            knowledge_pos: 5,
            detail: "search allowed".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-6".to_owned(),
            kind: TrajectoryEventKind::ToolSearchCompleted,
            logical_pos: 6,
            parent_ids: vec!["ev-5".to_owned()],
            valid_pos: 6,
            knowledge_pos: 6,
            detail: "search ok".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-7".to_owned(),
            kind: TrajectoryEventKind::FinalAnswerProduced,
            logical_pos: 7,
            parent_ids: vec!["ev-4".to_owned(), "ev-6".to_owned()],
            valid_pos: 7,
            knowledge_pos: 7,
            detail: "done".to_owned(),
        },
    ];
    let clean_verdict = bridge
        .analyze_trajectory(&first.id, false, clean_events)
        .expect("analyze clean trajectory");
    assert!(clean_verdict.admitted, "clean trajectory must be admitted");
    assert_eq!(clean_verdict.temporal_status, "complete");
    assert_eq!(clean_verdict.replay_status, "valid");
    assert!(clean_verdict.forbidden_hits.is_empty());
    assert!(clean_verdict.proof_digest.starts_with("sha256:"));

    //    (b) Negative control: executed write event in read-only scenario -> REJECTED!
    let breach_events = vec![
        TrajectoryEvent {
            event_id: "ev-1".to_owned(),
            kind: TrajectoryEventKind::PromptSubmitted,
            logical_pos: 1,
            parent_ids: Vec::new(),
            valid_pos: 1,
            knowledge_pos: 1,
            detail: "prompt".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-2".to_owned(),
            kind: TrajectoryEventKind::ToolWriteExecuted,
            logical_pos: 2,
            parent_ids: vec!["ev-1".to_owned()],
            valid_pos: 2,
            knowledge_pos: 2,
            detail: "illegal write".to_owned(),
        },
    ];
    let breach_verdict = bridge
        .analyze_trajectory(&first.id, false, breach_events)
        .expect("analyze breach trajectory");
    assert!(
        !breach_verdict.admitted,
        "trajectory with executed write MUST be rejected by Bi-Temporal Causality"
    );
    assert!(
        breach_verdict
            .forbidden_hits
            .iter()
            .any(|h| h == "tool_write_executed")
    );

    // 4. Verify receipt parsing, tier classification, ground-truth scoring, and TOML config.
    let sample = "generation: abc\nelapsed_us: 42\nindexed_files: 1556\nmatched_files: 1316\nedb_coverage: 1316/1556 (84%, native_ast_files: 1316, facts: 18412, facts_truncated: false)\nmode: ascent\nrelation: answer\nresults: 14\nreturned: 14\ntruncated: false\n";
    let receipt = parse_search_receipt(sample);
    assert_eq!(receipt.total_count, Some(14));
    assert_eq!(
        classify_tool_call("workspace_search", "(brute-search \"ModelGateway\")"),
        QueryTierKind::L0Brute
    );
    assert_eq!(
        classify_tool_call("structural_query", "(ascent-fragment ((answer x) <-- (rust_struct _ _ x _)))"),
        QueryTierKind::L1Ascent
    );
    assert_eq!(
        classify_tool_call("constraint_solve", "(search-choice-program (use rust) (choice tier (m l)) (solve tier))"),
        QueryTierKind::L2Dusa
    );
    let (matched, missed) = score_ground_truth(
        "WeaverModelGateway in crates/marlin-agent-stream/src/gateway.rs and RuntimeEdgeModelGateway in crates/marlin-agent-runtime/src/model_gateway.rs plus StaticGateway",
        &first.checkpoints,
    );
    assert_eq!(matched, first.checkpoints.len());
    assert!(missed.is_empty());

    let cfg = LiveWeaverTomlConfig::load(&root.join("crates/marlin-agent-tui/example.toml"))
        .expect("load example.toml");
    let route = cfg
        .resolve_route_with("default", 65_536, |key| match key {
            "ANTHROPIC_API_KEY" => Some("test-key".to_owned()),
            "ANTHROPIC_BASE_URL" => Some("https://api.deepseek.com/anthropic".to_owned()),
            _ => None,
        })
        .expect("resolve default route");
    assert_eq!(route.endpoint.provider.as_str(), "deepseek");
    assert_eq!(route.endpoint.model.as_str(), "deepseek-flash");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live Weaver LLM natural-language search evaluation suite (requires ANTHROPIC_API_KEY and ANTHROPIC_BASE_URL)"]
async fn live_weaver_natural_language_search_eval_suite() {
    let root = repo_root();
    let config_path = env::var_os("MARLIN_LIVE_EVAL_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("crates/marlin-agent-tui/example.toml"));
    let model_alias = env::var("MARLIN_LIVE_EVAL_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "default".to_owned());
    let suite_selector = env::var("MARLIN_LIVE_EVAL_SUITE")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let case_filter = env::var("MARLIN_LIVE_EVAL_CASE")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let report_path = env::var_os("MARLIN_LIVE_EVAL_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/live-weaver-search-eval-report.md"));

    let budget = LiveEvalBudget::from_env();
    let cfg = LiveWeaverTomlConfig::load(&config_path).expect("load Weaver TOML config");
    let route = cfg
        .resolve_route(&model_alias, budget.max_stream_bytes)
        .expect("resolve live Weaver route (ensure ANTHROPIC_API_KEY & ANTHROPIC_BASE_URL are set)");

    let scheme_bridge = HarnessSchemeBridge::from_workspace(&root)
        .expect("locate compiled Marlin scheme/ directory");
    let suite = scheme_bridge
        .project_harness_suite_named(suite_selector.as_deref())
        .expect("project declarative Scheme TestingSuite");
    let session = LiveWeaverEvalSession::open(&root, &scheme_bridge, route, budget)
        .expect("open headless LiveWeaverEvalSession");

    let offered_summary: Vec<String> = session
        .offered_definitions()
        .iter()
        .map(|d| format!("{} ({} chars desc)", d.name.as_str(), d.description.len()))
        .collect();
    eprintln!(
        "[harness] suite={} model={}/{} indexed_files={} hydrated_files={} tools=[{}]",
        suite.suite_id,
        session.route().endpoint.provider.as_str(),
        session.route().endpoint.model.as_str(),
        session.indexed_files(),
        session.hydrated_files(),
        offered_summary.join(", "),
    );

    let selected: Vec<_> = suite
        .scenarios
        .iter()
        .filter(|s| {
            case_filter
                .as_deref()
                .is_none_or(|needle| s.id.contains(needle))
        })
        .collect();
    assert!(
        !selected.is_empty(),
        "no Scheme evaluation scenarios matched MARLIN_LIVE_EVAL_CASE={case_filter:?}"
    );

    let mut reports = Vec::with_capacity(selected.len());
    for scenario in selected {
        eprintln!(
            "\n[harness] >>> Running Scheme scenario `{}` ({}) [gov={}] ...",
            scenario.id, scenario.title, scenario.governance.policy_id
        );
        let report = session
            .run_scenario(scenario)
            .await
            .expect("run live Weaver scenario");
        eprintln!(
            "[harness] <<< Done `{}`: rounds={} search_calls={} bash={} errors={} zero_hits={} causal_admitted={} spans={} gt={}/{} wall={}ms gaps={}",
            report.scenario_id,
            report.rounds_used,
            report
                .tool_calls
                .iter()
                .filter(|c| c.governance.capability == "search")
                .count(),
            report
                .tool_calls
                .iter()
                .filter(|c| c.tool_name == "bash")
                .count(),
            report
                .tool_calls
                .iter()
                .filter(|c| c.status == "failed")
                .count(),
            report
                .tool_calls
                .iter()
                .filter(|c| c.status == "completed" && c.receipt.total_count == Some(0))
                .count(),
            report.temporal_causality.admitted,
            report.telemetry_spans.len(),
            report.matched_checkpoints,
            report.total_checkpoints,
            report.wall_ms,
            report.gaps.len(),
        );
        reports.push(report);
    }

    let markdown = render_live_eval_markdown_report(
        &suite.suite_id,
        &session.route().endpoint,
        session.indexed_files(),
        session.hydrated_files(),
        &reports,
    );
    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&report_path, &markdown).expect("write eval markdown report");
    eprintln!(
        "\n[harness] Full report written to {}\n\n{markdown}",
        report_path.display()
    );

    for r in &reports {
        assert!(
            r.temporal_causality.admitted,
            "scenario `{}` failed Bi-Temporal Causality governance proof: {:?}",
            r.scenario_id, r.temporal_causality
        );
        if !r.write_allowed {
            assert!(
                r.governance_receipts
                    .iter()
                    .all(|g| !(g.is_write && g.permitted)),
                "scenario `{}` permitted a forbidden write tool call!",
                r.scenario_id
            );
        }
    }
    let any_aborted = reports.iter().any(|r| r.aborted_reason.is_some());
    assert!(
        !any_aborted,
        "one or more live Weaver search scenarios aborted; inspect report at {}",
        report_path.display()
    );
}
