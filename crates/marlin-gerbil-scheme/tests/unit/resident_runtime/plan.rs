use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilCommandProfile, GerbilResidentRuntimePlan,
    GerbilResidentRuntimeSessionMode,
};
use tempfile::Builder;

#[test]
fn resident_runtime_plan_defaults_to_disabled_process_reuse() {
    let plan = GerbilResidentRuntimePlan::disabled("/tmp/marlin-gerbil-loadpath");

    assert_eq!(
        plan.session_mode,
        GerbilResidentRuntimeSessionMode::Disabled
    );
    assert!(!plan.requires_process_reuse());
    assert!(!plan.isolates_state());
    assert!(plan.session_id.is_none());
    assert_eq!(plan.command_profile.args, vec![GERBIL_ADAPTER_MODULE]);
    assert!(plan.command_profile.env.contains_key(GERBIL_LOADPATH_ENV));
}

#[test]
fn resident_runtime_plan_distinguishes_shared_forked_and_isolated_sessions() {
    let shared =
        GerbilResidentRuntimePlan::shared_context("/tmp/marlin-gerbil-loadpath", "shared-session");
    let forked =
        GerbilResidentRuntimePlan::forked_context("/tmp/marlin-gerbil-loadpath", "forked-session");
    let isolated = GerbilResidentRuntimePlan::isolated_session(
        "/tmp/marlin-gerbil-loadpath",
        "isolated-session",
    )
    .with_command_profile(GerbilCommandProfile::new("/opt/gerbil/bin/gxi").arg("-e"));

    assert_eq!(
        shared.session_mode,
        GerbilResidentRuntimeSessionMode::SharedContext
    );
    assert_eq!(
        forked.session_mode,
        GerbilResidentRuntimeSessionMode::ForkedContext
    );
    assert_eq!(
        isolated.session_mode,
        GerbilResidentRuntimeSessionMode::IsolatedSession
    );
    assert!(shared.requires_process_reuse());
    assert!(forked.requires_process_reuse());
    assert!(isolated.requires_process_reuse());
    assert!(!shared.isolates_state());
    assert!(!forked.isolates_state());
    assert!(isolated.isolates_state());
    assert_eq!(
        isolated
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("isolated-session")
    );
    assert_eq!(isolated.command_profile.program, "/opt/gerbil/bin/gxi");
    assert_eq!(isolated.command_profile.args, vec!["-e"]);
}

#[test]
fn resident_runtime_prepare_writes_assets_and_emits_receipt() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "resident-session")
        .prepare()
        .expect("prepare resident runtime");
    let receipt = handle.receipt();

    assert_eq!(
        receipt.session_mode,
        GerbilResidentRuntimeSessionMode::SharedContext
    );
    assert_eq!(
        receipt
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("resident-session")
    );
    assert!(receipt.process_reuse_required);
    assert!(!receipt.state_isolated);
    assert_eq!(receipt.written_asset_count, handle.written_assets().len());
    assert!(receipt.written_asset_count > 0);
    assert!(root.path().join("gerbil.pkg").exists());
}
