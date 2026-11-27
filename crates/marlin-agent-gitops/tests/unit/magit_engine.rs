// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{collections::BTreeMap, fs, path::Path, process::Command};

use marlin_agent_gitops::magit::{
    AitiaG2Finding, AitiaLiveReport, AitiaMarlinGrpcClient, MagitAction, MagitAuditConcern,
    MagitAuditRisk, MagitBackendKind, MagitEngine, MagitFileSection, MagitFormalStatus,
    MagitJointAuditRecord, build_single_hunk_patch, fuse_hunk_evidence, parse_unified_diff,
};

fn run_cmd(dir: &Path, program: &str, args: &[&str]) {
    let out = Command::new(program)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("command should spawn");
    assert!(
        out.status.success(),
        "{program} {:?} failed:\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn evidence_fusion_is_honest_across_offline_aitia_findings_formal_and_human_verdicts() {
    let sample = "\
diff --git a/crates/auth/src/gate.rs b/crates/auth/src/gate.rs
index 1111111..2222222 100644
--- a/crates/auth/src/gate.rs
+++ b/crates/auth/src/gate.rs
@@ -10,3 +10,5 @@ fn verify(token: &str) -> bool {
     if token.is_empty() {
         return false;
     }
+    let _bypass = false;
+    true
 }
";
    let mut parsed = parse_unified_diff(sample, MagitFileSection::Unstaged);
    let hunks = parsed.get_mut("crates/auth/src/gate.rs").unwrap();
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].concern, MagitAuditConcern::SecurityBoundary);
    // Initial parse never fabricates HIGH risk before real evidence arrives.
    assert_eq!(hunks[0].risk, MagitAuditRisk::Unassessed);

    // 1. Offline Aitia -> explicitly Unassessed (never fake HIGH).
    let offline = AitiaLiveReport::offline("sidecar stopped");
    fuse_hunk_evidence(
        &mut hunks[0],
        &offline,
        MagitFormalStatus::None,
        None,
        &[],
        false,
    );
    assert_eq!(hunks[0].risk, MagitAuditRisk::Unassessed);

    // 2. Real Lambda Aitia G2 finding hitting line 13 -> elevates to High with cited rule!
    let with_finding = AitiaLiveReport {
        online: true,
        transport: "grpc".to_owned(),
        decision: "BLOCK".to_owned(),
        engine_owner: "scheme-poo-ascent-framework".to_owned(),
        report_digest: Some("sha256:abcdef123456".to_owned()),
        causal_acyclic: Some(true),
        total_findings: 1,
        counts_by_class: BTreeMap::from([("silent-fallback".to_owned(), 1)]),
        findings: vec![AitiaG2Finding {
            surface: "unstaged".to_owned(),
            subdomain: "security".to_owned(),
            hallucination_class: "silent-fallback".to_owned(),
            rule: "g2/no-auth-bypass".to_owned(),
            file: "crates/auth/src/gate.rs".to_owned(),
            line: 13,
            excerpt: "let _bypass = false;".to_owned(),
        }],
        note: "Aitia gRPC BLOCK".to_owned(),
    };
    fuse_hunk_evidence(
        &mut hunks[0],
        &with_finding,
        MagitFormalStatus::None,
        None,
        &[],
        false,
    );
    assert_eq!(hunks[0].risk, MagitAuditRisk::High);
    assert!(hunks[0].risk_reason.contains("silent-fallback/g2/no-auth-bypass"));

    // 3. Clean Aitia ADMIT + verified formal evidence + Human-AI audited -> Info!
    let clean_admit = AitiaLiveReport {
        online: true,
        transport: "grpc".to_owned(),
        decision: "ADMIT".to_owned(),
        engine_owner: "scheme-poo-ascent-framework".to_owned(),
        report_digest: Some("sha256:999999888888".to_owned()),
        causal_acyclic: Some(true),
        total_findings: 0,
        counts_by_class: BTreeMap::new(),
        findings: Vec::new(),
        note: "Aitia gRPC ADMIT".to_owned(),
    };
    let joint = vec![MagitJointAuditRecord {
        hunk_id: hunks[0].hunk_id.clone(),
        actor: "joint".to_owned(),
        verdict: "audited".to_owned(),
        rationale: "Reviewed token check invariant".to_owned(),
        context_digest: Some("sha256:999999888888".to_owned()),
    }];
    fuse_hunk_evidence(
        &mut hunks[0],
        &clean_admit,
        MagitFormalStatus::Verified,
        Some("dev/quint/gate.qnt"),
        &joint,
        false,
    );
    assert_eq!(hunks[0].risk, MagitAuditRisk::Info);
}

#[tokio::test]
async fn magit_engine_hunk_stage_unstage_stash_and_live_aitia_grpc() {
    let dir = tempfile::tempdir().expect("tempdir");
    run_cmd(dir.path(), "git", &["init", "-b", "main"]);
    run_cmd(dir.path(), "git", &["config", "user.name", "Marlin Test"]);
    run_cmd(
        dir.path(),
        "git",
        &["config", "user.email", "test@marlin.local"],
    );
    let initial = (1..=25)
        .map(|i| format!("line {i}\n"))
        .collect::<String>();
    fs::write(dir.path().join("multi.rs"), &initial).unwrap();
    run_cmd(dir.path(), "git", &["add", "multi.rs"]);
    run_cmd(dir.path(), "git", &["commit", "-m", "feat: initial multi"]);

    // Edit TWO separate regions so Git emits 2 distinct hunks in multi.rs!
    let modified = (1..=25)
        .map(|i| match i {
            2 => "line 2 edited hunk one\n".to_owned(),
            22 => "line 22 edited hunk two\n".to_owned(),
            _ => format!("line {i}\n"),
        })
        .collect::<String>();
    fs::write(dir.path().join("multi.rs"), modified).unwrap();

    let engine = MagitEngine::discover(dir.path()).await.unwrap();
    assert_eq!(engine.backend(), MagitBackendKind::GitNative);

    // Exercise live Lambda Aitia gRPC gateway if ~/.local/bin/aitia is installed!
    let aitia_client = AitiaMarlinGrpcClient::from_env();
    let live_report = aitia_client.audit_diff_defense(dir.path(), "HEAD").await;
    let snap = engine
        .snapshot_with_evidence(1, &live_report, &[])
        .await
        .unwrap();
    assert_eq!(snap.unstaged.len(), 1);
    assert_eq!(snap.unstaged[0].hunks.len(), 2, "expected 2 separate hunks");

    // Stage ONLY hunk 1 via real Magit hunk-level `git apply --cached`!
    let hunk1_patch = build_single_hunk_patch(&snap.unstaged[0].hunks[0]);
    let r_stage_hunk = engine
        .execute(MagitAction::StageHunk {
            path: "multi.rs".to_owned(),
            patch: hunk1_patch,
        })
        .await
        .unwrap();
    assert!(r_stage_hunk.ok);

    let snap2 = engine
        .snapshot_with_evidence(2, &live_report, &[])
        .await
        .unwrap();
    assert_eq!(snap2.staged.len(), 1);
    assert_eq!(snap2.staged[0].hunks.len(), 1, "only hunk 1 should be staged");
    assert_eq!(snap2.unstaged.len(), 1);
    assert_eq!(
        snap2.unstaged[0].hunks.len(),
        1,
        "hunk 2 should remain unstaged"
    );

    // Unstage hunk 1 cleanly.
    let staged_patch = build_single_hunk_patch(&snap2.staged[0].hunks[0]);
    engine
        .execute(MagitAction::UnstageHunk {
            path: "multi.rs".to_owned(),
            patch: staged_patch,
        })
        .await
        .unwrap();

    // Test real Magit stash push -> stash list -> stash pop!
    engine
        .execute(MagitAction::StashPush {
            message: Some("wip-magit-test".to_owned()),
        })
        .await
        .unwrap();
    let snap3 = engine
        .snapshot_with_evidence(3, &live_report, &[])
        .await
        .unwrap();
    assert!(snap3.unstaged.is_empty());
    assert_eq!(snap3.stashes.len(), 1);
    assert!(snap3.stashes[0].summary.contains("wip-magit-test"));

    engine.execute(MagitAction::StashPop).await.unwrap();
    let snap4 = engine
        .snapshot_with_evidence(4, &live_report, &[])
        .await
        .unwrap();
    assert_eq!(snap4.unstaged.len(), 1);
    assert!(snap4.stashes.is_empty());

    // Commit staged changes and verify live Aitia gRPC telemetry when installed.
    engine.execute(MagitAction::StageAll).await.unwrap();
    engine
        .execute(MagitAction::Commit {
            message: "feat: commit both hunks".to_owned(),
            amend: false,
        })
        .await
        .unwrap();
    if std::path::Path::new(&format!(
        "{}/.local/bin/aitia",
        std::env::var("HOME").unwrap_or_default()
    ))
    .is_file()
    {
        assert!(
            live_report.online,
            "expected live Lambda Aitia gRPC audit to succeed when ~/.local/bin/aitia is installed: {}",
            live_report.note
        );
        assert_eq!(live_report.transport, "grpc");
    }
}

#[tokio::test]
async fn magit_engine_colocated_jj_detects_change_id_oplog_and_undo() {
    let has_jj = Command::new("/opt/homebrew/bin/jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Command::new("jj")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !has_jj {
        return;
    }

    let jj_bin = if std::path::Path::new("/opt/homebrew/bin/jj").is_file() {
        "/opt/homebrew/bin/jj"
    } else {
        "jj"
    };

    let dir = tempfile::tempdir().expect("tempdir");
    run_cmd(dir.path(), jj_bin, &["git", "init", "--colocate"]);
    run_cmd(
        dir.path(),
        jj_bin,
        &["config", "set", "--repo", "user.name", "Marlin JJ"],
    );
    run_cmd(
        dir.path(),
        jj_bin,
        &["config", "set", "--repo", "user.email", "jj@marlin.local"],
    );
    fs::write(dir.path().join("README.md"), "# Colocated JJ\n").unwrap();

    let engine = MagitEngine::discover(dir.path()).await.unwrap();
    assert_eq!(engine.backend(), MagitBackendKind::JjColocated);

    let offline = AitiaLiveReport::offline("unit test");
    let s1 = engine.snapshot_with_evidence(1, &offline, &[]).await.unwrap();
    assert!(s1.change_id.is_some());
    assert!(!s1.op_log.is_empty());

    let r_desc = engine
        .execute(MagitAction::JjDescribe {
            message: "feat(jj): describe colocated change".to_owned(),
        })
        .await
        .unwrap();
    assert!(r_desc.ok);

    let s2 = engine.snapshot_with_evidence(2, &offline, &[]).await.unwrap();
    assert!(s2.head_description.contains("describe colocated change"));

    let r_undo = engine.execute(MagitAction::JjUndo).await.unwrap();
    assert!(r_undo.ok);
}
