// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin Agent Core <-> `aitia serve-marlin-grpc` Integration & Live LLM Tool-Calling Eval.
//!
//! Offline contract & gRPC sidecar test (zero API keys, zero network):
//!   `cargo test --offline -p marlin-agent-harness --features live-weaver --test live_weaver_aitia_review_eval`
//!
//! Live API LLM tool-calling eval (`<= 4` rounds, zero pre-seeded DSL coaching):
//!   `set -a && source .env && set +a && cargo test --offline -p marlin-agent-harness --features live-weaver --test live_weaver_aitia_review_eval live_weaver_aitia_review_tool_calling_eval -- --ignored --nocapture`

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use marlin_agent_harness::live_weaver::aitia_grpc::{
    AitiaMagitReviewSnapshot, ManagedAitiaSidecarServer, aitia_review_readonly_governance_policy,
    run_live_aitia_review_eval,
};
use marlin_agent_harness::live_weaver::{
    CompiledHarnessGovernanceGate, HarnessSchemeBridge, LiveEvalBudget, LiveWeaverTomlConfig,
    TrajectoryEvent, TrajectoryEventKind,
};
use serde_json::json;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize marlin repo root")
}

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn create_sample_defective_git_repo(dir: &Path) {
    run_git(dir, &["init", "-b", "main"]);
    run_git(dir, &["config", "user.name", "Marlin Eval"]);
    run_git(dir, &["config", "user.email", "eval@marlin.local"]);
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).expect("mkdir src");
    fs::write(
        src_dir.join("auth_gate.rs"),
        "pub fn check_token(tok: &str) -> Result<bool, &'static str> {\n\
         \x20   assert!(!tok.is_empty());\n\
         \x20   if tok.trim().is_empty() {\n\
         \x20       return Err(\"blank token\");\n\
         \x20   }\n\
         \x20   Ok(true)\n\
         }\n",
    )
    .expect("write initial auth_gate.rs");
    fs::write(
        src_dir.join("session_store.rs"),
        "pub fn load_epoch(raw: &str) -> Result<u64, &'static str> {\n\
         \x20   assert!(!raw.is_empty());\n\
         \x20   raw.parse::<u64>().map_err(|_| \"invalid epoch\")\n\
         }\n",
    )
    .expect("write initial session_store.rs");
    run_git(dir, &["add", "."]);
    run_git(
        dir,
        &[
            "commit",
            "-m",
            "feat(auth): initial token gate and session epoch",
        ],
    );

    // Introduce real multi-file DE G2 defects: deleted assertion + silent fallback `unwrap_or_default()`.
    fs::write(
        src_dir.join("auth_gate.rs"),
        "pub fn check_token(tok: Option<&str>) -> bool {\n\
         \x20   let raw = tok.unwrap_or_default();\n\
         \x20   !raw.is_empty()\n\
         }\n",
    )
    .expect("write defective auth_gate.rs");
    fs::write(
        src_dir.join("session_store.rs"),
        "pub fn load_epoch(raw: &str) -> u64 {\n\
         \x20   raw.parse::<u64>().unwrap_or(0)\n\
         }\n",
    )
    .expect("write defective session_store.rs");
}

#[test]
fn offline_marlin_aitia_grpc_cedar_cas_and_bitemporal_contract() {
    let root = repo_root();
    let aitia_bin = ManagedAitiaSidecarServer::resolve_aitia_binary()
        .expect("aitia binary must be installed at ~/.local/bin/aitia or AITIA_BIN");

    // 1. Verify fail-closed Cedar read-only governance permits ONLY `aitia_review_step`
    //    and denies `bash`, `write_file`, and capability spoofing.
    let gate = CompiledHarnessGovernanceGate::compile(aitia_review_readonly_governance_policy())
        .expect("compile aitia read-only Cedar gate");
    let allow_review =
        gate.evaluate_tool_call("aitia-offline", "c-1", "aitia_review_step", Some("review"));
    assert!(allow_review.permitted && !allow_review.is_write);
    assert_eq!(allow_review.cedar_decision, "Allow");

    let deny_bash = gate.evaluate_tool_call("aitia-offline", "c-2", "bash", Some("review"));
    assert!(!deny_bash.permitted && deny_bash.is_write);
    assert_eq!(deny_bash.cedar_decision, "Forbid");

    let deny_write = gate.evaluate_tool_call("aitia-offline", "c-3", "write_file", Some("review"));
    assert!(!deny_write.permitted && deny_write.is_write);

    let deny_unlisted =
        gate.evaluate_tool_call("aitia-offline", "c-4", "workspace_search", Some("search"));
    assert!(!deny_unlisted.permitted);

    // 2. Spawn process-isolated `aitia serve-marlin-grpc` with fresh Bearer token.
    let token = "marlin-offline-bearer-9f8e7d6c5b4a";
    let server =
        ManagedAitiaSidecarServer::spawn(&aitia_bin, token).expect("spawn aitia serve-marlin-grpc");
    let client = server.client();

    // Negative control: wrong Bearer token MUST fail closed with UNAUTHENTICATED (status 16).
    let bad_auth = client
        .call_rpc_raw("Health", &json!({}), Some("wrong-token"))
        .expect("raw Health call");
    assert_eq!(bad_auth.grpc_status, 16);

    // Health + single-source-of-truth `GetToolSchema` -> Marlin `ModelToolDefinition`.
    let health = client.health().expect("Health RPC");
    assert_eq!(health["status"].as_str(), Some("SERVING"));
    assert_eq!(health["actionAuthorized"].as_bool(), Some(false));

    let tool_def = client.get_tool_definition().expect("GetToolSchema RPC");
    assert_eq!(tool_def.name.as_str(), "aitia_review_step");
    assert!(tool_def.description.contains("aitia-review-plan"));
    assert!(tool_def.parameters["properties"]["planSexp"].is_object());

    // 3. Create temp Git repo with real DE G2 defects and run Turn 1 -> Turn 2 -> Cycle over gRPC!
    let tmp = tempfile::tempdir().expect("tempdir");
    create_sample_defective_git_repo(tmp.path());
    let repo_str = tmp.path().to_string_lossy().into_owned();

    // Turn 1: zero-knowledge discovery `{ "repo": ... }`
    let turn1 = client
        .step_review(&json!({ "repo": repo_str }))
        .expect("Turn 1 StepReview discovery");
    assert_eq!(turn1["actionAuthorized"].as_bool(), Some(false));
    assert_eq!(
        turn1["sharedPrefix"]["sharedPrefixCasHit"].as_bool(),
        Some(false)
    );
    assert!(
        turn1["sharedPrefix"]["totalDiffFindings"]
            .as_u64()
            .unwrap_or(0)
            >= 2
    );
    assert!(
        turn1["dslGrammar"].is_array(),
        "Turn 1 must emit dslGrammar"
    );
    assert!(
        turn1["sexpPlanTemplate"].is_string(),
        "Turn 1 must emit sexpPlanTemplate"
    );
    assert!(
        !turn1["recommendedFormalModels"]
            .as_array()
            .expect("recommendedFormalModels")
            .is_empty()
    );

    // Turn 2: declarative `(aitia-review-plan ...)` S-expr -> hits cross-round FunFlow CAS,
    // converges 2 -> 1 unique-deduction world, and omits bulky teaching kit!
    let plan_sexp = format!(
        "(aitia-review-plan\n\
           (scenario \"marlin-auth-gate-review\")\n\
           (repo \"{repo_str}\")\n\
           (depends-on \"audit-diff\" \"verify-formal\")\n\
           (open-branches \"hyp-real-g2-defect\" \"hyp-false-positive\")\n\
           (probes\n\
             (probe \"hyp-real-g2-defect\" supports \"auth_gate.rs deleted assert and added unwrap_or_default\")\n\
             (probe \"hyp-false-positive\" refutes \"gix diff proves unwrap_or_default is live code\")))"
    );
    let turn2 = client
        .step_review(&json!({ "repo": repo_str, "planSexp": plan_sexp }))
        .expect("Turn 2 StepReview S-expr");
    assert_eq!(turn2["actionAuthorized"].as_bool(), Some(false));
    assert_eq!(
        turn2["sharedPrefix"]["sharedPrefixCasHit"].as_bool(),
        Some(true),
        "Turn 2 over unchanged diff MUST hit cross-round FunFlow CAS"
    );
    assert_eq!(turn2["planValidation"]["validated"].as_bool(), Some(true));
    assert_eq!(turn2["planValidation"]["acyclic"].as_bool(), Some(true));
    assert_eq!(
        turn2["ascentChoiceSolution"]["choiceStatus"].as_str(),
        Some("unique-deduction")
    );
    assert_eq!(
        turn2["ascentChoiceSolution"]["worldCount"].as_u64(),
        Some(1)
    );
    assert!(
        turn2.get("dslGrammar").is_none(),
        "Turn 2 valid plan must omit bulky dslGrammar to save LLM tokens"
    );
    assert_eq!(
        turn2["compactChoices"].as_bool(),
        Some(true),
        "Turn 2 valid plan must enable compactChoices (omitting choices[*].template)"
    );
    assert!(
        turn2["choices"]
            .as_array()
            .expect("choices")
            .iter()
            .all(|c| c.get("template").is_none()),
        "Turn 2 compactChoices must omit nested template objects"
    );
    let turn1_bytes = serde_json::to_string(&turn1).expect("t1 json").len();
    let turn2_bytes = serde_json::to_string(&turn2).expect("t2 json").len();
    assert!(
        turn2_bytes < turn1_bytes,
        "Turn 2 compact response ({turn2_bytes}B) must be smaller than Turn 1 discovery ({turn1_bytes}B)"
    );

    // Turn 2b: Machine-executed Quint 0.33 ephemeral overlay over gRPC:
    //   (a) vacuous overlay without @countermodel MUST fail closed!
    let vacuous_qnt = "module VacuousAuth {\n  var ok: bool\n  action init = ok' = true\n  run trivialTest = init.expect(ok)\n}\n";
    let turn2_vac = client
        .step_review(&json!({
            "repo": repo_str,
            "planSexp": plan_sexp,
            "quintOverlayName": "VacuousAuth.qnt",
            "quintOverlaySource": vacuous_qnt,
        }))
        .expect("Turn 2b vacuous Quint overlay");
    assert_eq!(
        turn2_vac["formalOverlayExecution"]["passed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        turn2_vac["formalOverlayExecution"]["countermodelVerified"].as_bool(),
        Some(false)
    );
    assert_eq!(
        turn2_vac["formalOverlayExecution"]["storedInSidecarVault"].as_bool(),
        Some(true)
    );

    //   (b) real MR05 overlay from Turn 1's own quintOverlayTemplate MUST fail closed if vaultDir
    //       is inside the repo, and pass + archive when vaultDir is outside the repo!
    let external_vault = tempfile::tempdir().expect("external vault tempdir");
    let vault_str = external_vault.path().to_string_lossy().into_owned();
    let qnt_template = turn1["quintOverlayTemplate"]
        .as_str()
        .expect("Turn 1 must emit quintOverlayTemplate");
    let illegal_vault = tmp.path().join(".illegal-vault");
    let turn2_bad_vault = client
        .step_review(&json!({
            "repo": repo_str,
            "planSexp": plan_sexp,
            "quintOverlayName": "AuthGateOverlay.qnt",
            "quintOverlaySource": qnt_template,
            "vaultDir": illegal_vault.to_string_lossy(),
        }))
        .expect("Turn 2b illegal vault inside repo");
    assert_eq!(
        turn2_bad_vault["formalOverlayExecution"]["passed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        turn2_bad_vault["formalOverlayExecution"]["storedInSidecarVault"].as_bool(),
        Some(false)
    );

    let turn2_qnt = client
        .step_review(&json!({
            "repo": repo_str,
            "planSexp": plan_sexp,
            "quintOverlayName": "AuthGateOverlay.qnt",
            "quintOverlaySource": qnt_template,
            "vaultDir": vault_str,
        }))
        .expect("Turn 2c valid Quint MR05 overlay");
    let qexec = &turn2_qnt["formalOverlayExecution"];
    assert_eq!(qexec["machineExecuted"].as_bool(), Some(true));
    assert_eq!(qexec["typecheckPassed"].as_bool(), Some(true));
    assert_eq!(qexec["testsPassed"].as_bool(), Some(true));
    assert_eq!(qexec["countermodelVerified"].as_bool(), Some(true));
    assert_eq!(qexec["passed"].as_bool(), Some(true));
    assert_eq!(qexec["storedInSidecarVault"].as_bool(), Some(true));
    assert_eq!(qexec["actionAuthorized"].as_bool(), Some(false));
    assert!(Path::new(qexec["archivedOverlayPath"].as_str().expect("qnt archive")).is_file());
    assert!(Path::new(qexec["archivedReceiptPath"].as_str().expect("qnt receipt")).is_file());
    assert!(!tmp.path().join("AuthGateOverlay.qnt").exists());

    // Turn 2d: Machine-executed Lean 4 kernel ephemeral overlay over gRPC:
    //   (a) `sorry` bypass MUST fail closed!
    let sorry_lean = "-- @countermodel\ntheorem fakeSafe : True := by sorry\ntheorem fakeCountermodel : False := by sorry\n";
    let turn2_lean_sorry = client
        .step_review(&json!({
            "repo": repo_str,
            "planSexp": plan_sexp,
            "leanOverlayName": "SorryAuth.lean",
            "leanOverlaySource": sorry_lean,
        }))
        .expect("Turn 2d sorry Lean overlay");
    assert_eq!(
        turn2_lean_sorry["leanOverlayExecution"]["passed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        turn2_lean_sorry["leanOverlayExecution"]["proofClean"].as_bool(),
        Some(false)
    );

    //   (b) real MR05 Lean 4 overlay from Turn 1's own leanOverlayTemplate MUST pass Lean 4 kernel
    //       AND archive into external_vault AND project cleanly into AitiaMagitReviewSnapshot!
    let lean_template = turn1["leanOverlayTemplate"]
        .as_str()
        .expect("Turn 1 must emit leanOverlayTemplate");
    let turn2_lean = client
        .step_review(&json!({
            "repo": repo_str,
            "planSexp": plan_sexp,
            "leanOverlayName": "AuthGateOverlay.lean",
            "leanOverlaySource": lean_template,
            "vaultDir": vault_str,
        }))
        .expect("Turn 2e valid Lean 4 MR05 overlay");
    let lexec = &turn2_lean["leanOverlayExecution"];
    assert_eq!(lexec["machineExecuted"].as_bool(), Some(true));
    assert_eq!(lexec["kernelPassed"].as_bool(), Some(true));
    assert_eq!(lexec["proofClean"].as_bool(), Some(true));
    assert_eq!(lexec["countermodelVerified"].as_bool(), Some(true));
    assert_eq!(lexec["passed"].as_bool(), Some(true));
    assert_eq!(lexec["storedInSidecarVault"].as_bool(), Some(true));
    assert_eq!(lexec["actionAuthorized"].as_bool(), Some(false));
    assert!(Path::new(lexec["archivedOverlayPath"].as_str().expect("lean archive")).is_file());
    assert!(Path::new(lexec["archivedReceiptPath"].as_str().expect("lean receipt")).is_file());
    assert!(!tmp.path().join("AuthGateOverlay.lean").exists());

    let magit_snap = AitiaMagitReviewSnapshot::from_review_response(&turn2_lean)
        .expect("project AitiaMagitReviewSnapshot");
    assert_eq!(magit_snap.world_count, 1);
    assert!(magit_snap.human_disposition_required);
    assert!(!magit_snap.action_authorized);
    assert!(magit_snap.lean_overlay.as_ref().is_some_and(|o| o.passed));

    // Turn 3 negative control: cyclic `(depends-on ...)` MUST fail closed with 0 worlds.
    let cyclic_sexp = format!(
        "(aitia-review-plan (repo \"{repo_str}\") (depends-on \"a\" \"b\") (depends-on \"b\" \"a\"))"
    );
    let turn3_cyc = client
        .step_review(&json!({ "repo": repo_str, "planSexp": cyclic_sexp }))
        .expect("Turn 3 cyclic StepReview");
    assert_eq!(
        turn3_cyc["planValidation"]["validated"].as_bool(),
        Some(false)
    );
    assert_eq!(
        turn3_cyc["planValidation"]["acyclic"].as_bool(),
        Some(false)
    );
    assert_eq!(
        turn3_cyc["ascentChoiceSolution"]["worldCount"].as_u64(),
        Some(0)
    );

    // 4. Verify Bi-Temporal Causality trajectory proof through Marlin's Scheme VM!
    let bridge = HarnessSchemeBridge::from_workspace(&root).expect("locate Marlin scheme/");
    let trajectory = vec![
        TrajectoryEvent {
            event_id: "ev-1".to_owned(),
            kind: TrajectoryEventKind::PromptSubmitted,
            logical_pos: 1,
            parent_ids: Vec::new(),
            valid_pos: 1,
            knowledge_pos: 1,
            detail: "aitia review prompt".to_owned(),
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
            kind: TrajectoryEventKind::GovernancePermitted,
            logical_pos: 3,
            parent_ids: vec!["ev-2".to_owned()],
            valid_pos: 3,
            knowledge_pos: 3,
            detail: "aitia_review_step permitted".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-4".to_owned(),
            kind: TrajectoryEventKind::ToolSearchCompleted,
            logical_pos: 4,
            parent_ids: vec!["ev-3".to_owned()],
            valid_pos: 4,
            knowledge_pos: 4,
            detail: "aitia_review_step ok".to_owned(),
        },
        TrajectoryEvent {
            event_id: "ev-5".to_owned(),
            kind: TrajectoryEventKind::FinalAnswerProduced,
            logical_pos: 5,
            parent_ids: vec!["ev-4".to_owned()],
            valid_pos: 5,
            knowledge_pos: 5,
            detail: "verified answer".to_owned(),
        },
    ];
    let verdict = bridge
        .analyze_trajectory("aitia-offline-review", false, trajectory)
        .expect("analyze trajectory in Marlin Scheme");
    assert!(
        verdict.admitted,
        "clean read-only aitia trajectory must be admitted"
    );
    assert!(verdict.proof_digest.starts_with("sha256:"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live Weaver API LLM tool-calling eval against aitia serve-marlin-grpc (requires ANTHROPIC_API_KEY and ANTHROPIC_BASE_URL)"]
async fn live_weaver_aitia_review_tool_calling_eval() {
    let root = repo_root();
    let aitia_bin = ManagedAitiaSidecarServer::resolve_aitia_binary()
        .expect("aitia binary must be installed at ~/.local/bin/aitia or AITIA_BIN");
    let config_path = env::var_os("MARLIN_LIVE_EVAL_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("crates/marlin-agent-tui/example.toml"));
    let model_alias = env::var("MARLIN_LIVE_EVAL_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "default".to_owned());

    let budget = LiveEvalBudget::from_env();
    let cfg = LiveWeaverTomlConfig::load(&config_path).expect("load Weaver TOML config");
    let route = cfg
        .resolve_route(&model_alias, budget.max_stream_bytes)
        .expect("resolve live Weaver route (source .env first)");

    let scheme_bridge = HarnessSchemeBridge::from_workspace(&root).expect("locate Marlin scheme/");

    // Spawn process-isolated `aitia serve-marlin-grpc` with fresh Bearer token.
    let server = ManagedAitiaSidecarServer::spawn(&aitia_bin, "marlin-live-eval-bearer-7a6b5c4d")
        .expect("spawn aitia serve-marlin-grpc");

    // Prepare a fresh, isolated Git repo containing real DE G2 defects so the live
    // LLM must discover the repo via `aitia_review_step`, author a valid
    // `(aitia-review-plan ...)` S-expr with competing hypotheses + probes + formal model,
    // and converge ASCENT worlds in <= 4 rounds!
    let tmp = tempfile::tempdir().expect("tempdir");
    let external_vault = tempfile::tempdir().expect("external vault tempdir");
    create_sample_defective_git_repo(tmp.path());
    let repo_str = tmp.path().to_string_lossy().into_owned();

    let prompt = format!(
        "Audit the uncommitted multi-file Git changes in repository `{repo_str}` using the `aitia_review_step` tool.\n\
         1. First discover the repository's live diff findings across `src/auth_gate.rs` and `src/session_store.rs`, DSL grammar, recommended DE formal models, `quintOverlayTemplate`, and `leanOverlayTemplate`.\n\
         2. Then submit a valid `(aitia-review-plan ...)` S-expression via `aitia_review_step` that defines two competing hypotheses (`open-branches`), attaches evidence probes (`probes`) to converge ASCENT to a single surviving world (`unique-deduction`), cites recommended DE formal models, AND attaches BOTH:\n\
            - an adapted executable Quint 0.33 ephemeral overlay (`quintOverlayName` + `quintOverlaySource` with `// @quint-extends`, `// @countermodel`, a positive invariant `run ...Test`, and a `*Countermodel*Test` trace), AND\n\
            - an adapted standalone Lean 4 ephemeral overlay (`leanOverlayName` + `leanOverlaySource` with `-- @lean-extends`, `-- @countermodel`, zero `sorry`/`axiom`, a positive invariant `theorem`, and a `*Countermodel*` refutation `theorem` proving the buggy fallback violates the invariant — kernel-verified live by `lean`).\n\
         3. If either `formalOverlayExecution.passed` or `leanOverlayExecution.passed` is false, fix the syntax/proof from its diagnostics in one follow-up call.\n\
         4. Finally summarize the exact findings in `src/auth_gate.rs` and `src/session_store.rs`, ASCENT convergence (`worldCount`), both machine-executed Quint + Lean 4 overlay results (and archived vault paths), and the recommended DE formal models."
    );

    let report = run_live_aitia_review_eval(
        &route,
        server.client(),
        &scheme_bridge,
        "live-aitia-review-tool-calling",
        tmp.path(),
        Some(external_vault.path()),
        &prompt,
        4,
    )
    .await
    .expect("run live aitia review eval");

    eprintln!(
        "\n=== LIVE MARLIN <-> AITIA GRPC TOOL-CALLING REPORT ===\n\
         model={}/{} rounds_used={} wall_ms={} calls={} causal_admitted={} spans={}",
        route.endpoint.provider.as_str(),
        route.endpoint.model.as_str(),
        report.rounds_used,
        report.wall_ms,
        report.tool_calls.len(),
        report.temporal_causality.admitted,
        report.telemetry_spans.len(),
    );
    for (idx, tc) in report.tool_calls.iter().enumerate() {
        eprintln!(
            "  call #{}: R{} tool={} status={} gov={} ms={} bytes={} compact={} cas_hit={} packs={:?} fmt={} valid={} acyclic={} choice={} worlds={} qnt=({},{},{},{}) lean=({},{},{},{},{}) recs={} grammar={}",
            idx + 1,
            tc.round,
            tc.tool_name,
            tc.status,
            tc.governance.cedar_decision,
            tc.elapsed_ms,
            tc.output_bytes,
            tc.compact_choices,
            tc.cas_hit,
            tc.rule_packs_applied,
            tc.plan_format,
            tc.plan_validated,
            tc.plan_acyclic,
            tc.choice_status,
            tc.world_count,
            tc.quint_overlay_executed,
            tc.quint_overlay_passed,
            tc.quint_countermodel_verified,
            tc.quint_passing_tests,
            tc.lean_overlay_executed,
            tc.lean_overlay_passed,
            tc.lean_proof_clean,
            tc.lean_countermodel_verified,
            tc.lean_theorem_count,
            tc.recommended_models_count,
            tc.dsl_grammar_emitted,
        );
    }
    eprintln!("\n--- FINAL ANSWER ---\n{}\n", report.final_answer);

    assert!(
        report.aborted_reason.is_none(),
        "live eval aborted: {:?}",
        report.aborted_reason
    );
    assert!(
        report.rounds_used <= 4,
        "must complete in <= 4 rounds, used {}",
        report.rounds_used
    );
    assert!(
        report.temporal_causality.admitted,
        "Bi-Temporal Causality rejected trajectory: {:?}",
        report.temporal_causality
    );
    assert!(
        report
            .governance_receipts
            .iter()
            .all(|g| !(g.is_write && g.permitted)),
        "Cedar governance must never permit write tools"
    );
    assert!(
        report.tool_calls.iter().all(|c| !c.action_authorized),
        "all aitia responses must preserve actionAuthorized=false"
    );
    assert!(
        report
            .tool_calls
            .iter()
            .any(|c| c.plan_format == "sexp-aitia-review-plan" && c.plan_validated),
        "live LLM MUST author at least one valid `(aitia-review-plan ...)` S-expr via self-discovery"
    );
    assert!(
        report
            .tool_calls
            .iter()
            .any(|c| c.choice_status == "unique-deduction" && c.world_count == 1),
        "live LLM MUST converge ASCENT Finite-Choice to unique-deduction (worldCount == 1)"
    );
    assert!(
        report
            .tool_calls
            .iter()
            .any(|c| c.cas_hit || c.rule_packs_applied.len() > 1),
        "Turn 2+ over unchanged repo must either hit cross-round FunFlow CAS (same packs) or compose a dynamic subdomain POO rule-pack (cache-sound invalidation)"
    );
    assert!(
        report.tool_calls.iter().any(|c| c.compact_choices),
        "Turn 2+ valid plan must use compactChoices to save LLM tokens"
    );
    assert!(
        report.tool_calls.iter().any(|c| {
            c.quint_overlay_executed
                && c.quint_overlay_passed
                && c.quint_countermodel_verified
                && c.quint_passing_tests >= 2
                && c.quint_archived_path
                    .as_deref()
                    .is_some_and(|p| Path::new(p).is_file())
        }),
        "live LLM MUST author, pass, and archive a machine-executed Quint 0.33 MR05 overlay in external vault"
    );
    assert!(
        report.tool_calls.iter().any(|c| {
            c.lean_overlay_executed
                && c.lean_overlay_passed
                && c.lean_proof_clean
                && c.lean_countermodel_verified
                && c.lean_theorem_count >= 2
                && c.lean_archived_path
                    .as_deref()
                    .is_some_and(|p| Path::new(p).is_file())
        }),
        "live LLM MUST author, pass, and archive a kernel-verified Lean 4 MR05 overlay in external vault"
    );
    assert!(
        report.tool_calls.iter().any(|c| c
            .magit_snapshot
            .as_ref()
            .is_some_and(|s| s.world_count == 1 && !s.action_authorized)),
        "production AitiaMagitReviewSnapshot must capture converged unique-deduction state with action_authorized=false"
    );
    assert!(
        report.final_answer.contains("auth_gate.rs")
            || report.final_answer.contains("unwrap_or_default"),
        "final answer must report the real finding in src/auth_gate.rs"
    );
}
