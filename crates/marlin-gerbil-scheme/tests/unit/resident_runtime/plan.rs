use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilCommandProfile, GerbilResidentRuntimePlan,
    GerbilResidentRuntimeSessionMode, GerbilResidentStrategyEventKind,
    GerbilResidentStrategyLaneStatus, GerbilResidentStrategyRequest,
    GerbilResidentStrategyRequestStatus,
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

#[test]
fn resident_runtime_strategy_service_declares_disabled_lanes_without_process_reuse() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-strategy-disabled-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::disabled(root.path())
        .prepare()
        .expect("prepare resident runtime");
    let receipt = handle.strategy_service_receipt();

    assert_eq!(
        receipt.session_mode,
        GerbilResidentRuntimeSessionMode::Disabled
    );
    assert!(!receipt.process_reuse_required);
    assert!(!receipt.state_isolated);
    assert_eq!(receipt.lane_count, 2);
    assert_eq!(receipt.ready_lane_count, 0);
    assert_eq!(receipt.disabled_lane_count, 2);
    assert_eq!(receipt.dynamic_replan_lane_count, 1);
    assert_eq!(receipt.policy_change_lane_count, 1);
    assert!(receipt.lanes.iter().all(|lane| {
        lane.status == GerbilResidentStrategyLaneStatus::Disabled && lane.command_profile.is_none()
    }));
}

#[test]
fn resident_runtime_strategy_service_rejects_requests_when_runtime_is_disabled() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-strategy-disabled-request-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::disabled(root.path())
        .prepare()
        .expect("prepare resident runtime");
    let service_plan = handle.strategy_service_plan();
    let receipt = service_plan.request_receipt(GerbilResidentStrategyRequest::new(
        "disabled-request",
        GerbilResidentStrategyEventKind::DynamicReplan,
    ));

    assert_eq!(
        receipt.status,
        GerbilResidentStrategyRequestStatus::RuntimeDisabled
    );
    assert_eq!(
        receipt.session_mode,
        GerbilResidentRuntimeSessionMode::Disabled
    );
    assert!(!receipt.process_reuse_required);
    assert!(receipt.session_id.is_none());
    assert!(receipt.child_id.is_none());
    assert!(receipt.process_health.is_none());
}

#[test]
fn resident_runtime_strategy_service_declares_ready_replan_and_policy_change_lanes() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-strategy-ready-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::isolated_session(root.path(), "strategy-session")
        .prepare()
        .expect("prepare resident runtime");
    let receipt = handle.strategy_service_receipt();

    assert_eq!(
        receipt.session_mode,
        GerbilResidentRuntimeSessionMode::IsolatedSession
    );
    assert!(receipt.process_reuse_required);
    assert!(receipt.state_isolated);
    assert_eq!(receipt.lane_count, 2);
    assert_eq!(receipt.ready_lane_count, 2);
    assert_eq!(receipt.disabled_lane_count, 0);
    assert_eq!(receipt.dynamic_replan_lane_count, 1);
    assert_eq!(receipt.policy_change_lane_count, 1);

    let dynamic_replan = receipt
        .lanes
        .iter()
        .find(|lane| lane.event_kind == GerbilResidentStrategyEventKind::DynamicReplan)
        .expect("dynamic replan lane");
    let policy_change = receipt
        .lanes
        .iter()
        .find(|lane| lane.event_kind == GerbilResidentStrategyEventKind::PolicyChange)
        .expect("policy change lane");

    assert_eq!(dynamic_replan.lane_id.as_str(), "dynamic-replan");
    assert_eq!(policy_change.lane_id.as_str(), "policy-change");
    assert_eq!(
        dynamic_replan.status,
        GerbilResidentStrategyLaneStatus::ReadyToServe
    );
    assert_eq!(
        policy_change.status,
        GerbilResidentStrategyLaneStatus::ReadyToServe
    );
    assert_eq!(
        dynamic_replan
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("strategy-session")
    );
    assert_eq!(
        dynamic_replan
            .command_profile
            .as_ref()
            .map(|profile| profile.args.clone()),
        Some(vec![GERBIL_ADAPTER_MODULE.to_string()])
    );
    assert_eq!(
        dynamic_replan
            .command_profile
            .as_ref()
            .and_then(|profile| profile.env.get(GERBIL_LOADPATH_ENV)),
        policy_change
            .command_profile
            .as_ref()
            .and_then(|profile| profile.env.get(GERBIL_LOADPATH_ENV))
    );
}

#[test]
fn resident_runtime_strategy_service_admits_only_matching_ready_lanes() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-strategy-request-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::isolated_session(root.path(), "strategy-session")
        .prepare()
        .expect("prepare resident runtime");
    let service_plan = handle.strategy_service_plan();

    let accepted = service_plan.request_receipt(
        GerbilResidentStrategyRequest::new(
            "accepted-request",
            GerbilResidentStrategyEventKind::DynamicReplan,
        )
        .with_policy_epoch(7),
    );
    assert_eq!(
        accepted.status,
        GerbilResidentStrategyRequestStatus::Accepted
    );
    assert_eq!(accepted.lane_id.as_str(), "dynamic-replan");
    assert_eq!(
        accepted
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("strategy-session")
    );
    assert_eq!(accepted.policy_epoch, Some(7));

    let mismatch = service_plan.request_receipt(
        GerbilResidentStrategyRequest::new(
            "mismatch-request",
            GerbilResidentStrategyEventKind::PolicyChange,
        )
        .with_lane_id("dynamic-replan"),
    );
    assert_eq!(
        mismatch.status,
        GerbilResidentStrategyRequestStatus::EventLaneMismatch
    );

    let unavailable = service_plan.request_receipt(
        GerbilResidentStrategyRequest::new(
            "unavailable-request",
            GerbilResidentStrategyEventKind::DynamicReplan,
        )
        .with_lane_id("missing-lane"),
    );
    assert_eq!(
        unavailable.status,
        GerbilResidentStrategyRequestStatus::LaneUnavailable
    );
}
