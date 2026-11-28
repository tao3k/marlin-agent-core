// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::fs;

use super::intent_case_artifact_support::{
    execute_vertical_receipt, gerbil_vertical_receipts, observed_span_source_for_vertical_receipt,
};
use marlin_agent_harness::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactBundleMaterializationError,
    materialize_gerbil_scripted_intent_case_artifact_bundle,
};

#[cfg(unix)]
#[test]
fn sm35_v4_intent_case_artifact_materialization_refuses_symlink_escapes() {
    let receipt = gerbil_vertical_receipts()
        .into_iter()
        .next()
        .expect("at least one Gerbil vertical case should exist");
    let execution_receipt = execute_vertical_receipt(&receipt);
    let observed_span = observed_span_source_for_vertical_receipt(&receipt);

    // Discover the exact relative layout materialized under a clean probe root.
    let probe_root = tempfile::tempdir().expect("create probe root");
    let probe_bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: probe_root.path().to_owned(),
            run_id: "sm35-probe".into(),
            vertical_trace: receipt.clone(),
            execution_receipt: execution_receipt.clone(),
            side_effect_replay_bundle: None,
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span.clone()),
            real_llm_case_receipt: None,
        },
    )
    .expect("probe materialization should succeed");
    let rel_bundle_dir = probe_bundle
        .bundle_root
        .strip_prefix(probe_root.path())
        .expect("bundle root lies beneath probe root")
        .to_path_buf();
    let first_component = rel_bundle_dir
        .components()
        .next()
        .expect("non-empty relative bundle dir");

    // 1. Intermediate directory symlink escape: plant symlink at output_root/<first_component> -> outside_victim
    let attack_root = tempfile::tempdir().expect("create attack root");
    let outside_victim = tempfile::tempdir().expect("create outside victim dir");
    symlink(outside_victim.path(), attack_root.path().join(first_component))
        .expect("plant intermediate dir symlink");

    let escape_err = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: attack_root.path().to_owned(),
            run_id: "sm35-probe".into(),
            vertical_trace: receipt.clone(),
            execution_receipt: execution_receipt.clone(),
            side_effect_replay_bundle: None,
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span.clone()),
            real_llm_case_receipt: None,
        },
    );
    assert!(
        matches!(
            escape_err,
            Err(IntentCaseArtifactBundleMaterializationError::UnsafeArtifactPath { .. })
        ),
        "SM35 V4: intermediate directory symlink escape MUST be refused with UnsafeArtifactPath, got {escape_err:?}"
    );
    let outside_entries: Vec<_> = fs::read_dir(outside_victim.path())
        .expect("read outside victim")
        .collect();
    assert!(
        outside_entries.is_empty(),
        "SM35 V4: zero files may be written into symlinked outside_victim directory"
    );

    // 2. Leaf symlink escape on manifest.receipt: pre-create real bundle dir + symlink manifest.receipt -> outside_file
    let leaf_root = tempfile::tempdir().expect("create leaf attack root");
    let leaf_outside = tempfile::tempdir().expect("create leaf outside victim");
    let outside_target = leaf_outside.path().join("host-secret.txt");
    fs::write(&outside_target, "ORIGINAL_HOST_SECRET").expect("seed outside target");
    let target_bundle_dir = leaf_root.path().join(&rel_bundle_dir);
    fs::create_dir_all(&target_bundle_dir).expect("create bundle dir");
    symlink(&outside_target, target_bundle_dir.join("manifest.receipt"))
        .expect("plant leaf symlink on manifest.receipt");

    let leaf_err = materialize_gerbil_scripted_intent_case_artifact_bundle(
        GerbilScriptedIntentCaseArtifactBundleRequest {
            output_root: leaf_root.path().to_owned(),
            run_id: "sm35-probe".into(),
            vertical_trace: receipt,
            execution_receipt,
            side_effect_replay_bundle: None,
            runtime_repair_receipt: None,
            observed_span_source: Some(observed_span),
            real_llm_case_receipt: None,
        },
    );
    assert!(
        leaf_err.is_err(),
        "SM35 V4: leaf symlink on manifest.receipt MUST be refused, got {leaf_err:?}"
    );
    assert_eq!(
        fs::read_to_string(&outside_target).expect("read outside target"),
        "ORIGINAL_HOST_SECRET",
        "SM35 V4: leaf symlink target outside output_root MUST remain untouched"
    );
}
