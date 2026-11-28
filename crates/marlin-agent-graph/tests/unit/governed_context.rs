// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_graph::{
    DetourMode, DisclosureDepth, GovernedContextCapsule, GovernedContextRejection, KvPrefixDigest,
    TaskActorId, admit_governed_context_capsule,
};

fn valid_capsule() -> GovernedContextCapsule {
    GovernedContextCapsule {
        task_id: TaskActorId::new("task-ctx-1").unwrap(),
        context_cut: 1,
        kv_prefix_digest: KvPrefixDigest::new("sha256:two-zone-frozen-prefix").unwrap(),
        disclosure_depth: DisclosureDepth::L1,
        has_active_defeater: false,
        quarantined_memory_count: 2,
        sandbox_admitted: true,
        cedar_handoff_valid: true,
        detour_mode: DetourMode::Mainline,
        auto_archive_enabled: true,
    }
}

#[test]
fn governed_context_gate_enforces_l3_defeater_floor_sandbox_cedar_and_blocker_detour() {
    let ok = admit_governed_context_capsule(valid_capsule()).unwrap();
    assert_eq!(ok.capsule().quarantined_memory_count, 2);

    // Active defeater below L3 must fail closed.
    let mut bad_defeater = valid_capsule();
    bad_defeater.has_active_defeater = true;
    bad_defeater.disclosure_depth = DisclosureDepth::L2;
    assert_eq!(
        admit_governed_context_capsule(bad_defeater),
        Err(GovernedContextRejection::ActiveDefeaterRequiresL3Disclosure)
    );

    // Active defeater WITH L3 is admitted.
    let mut ok_defeater = valid_capsule();
    ok_defeater.has_active_defeater = true;
    ok_defeater.disclosure_depth = DisclosureDepth::L3;
    assert!(admit_governed_context_capsule(ok_defeater).is_ok());

    // Lethal-Trifecta sandbox denial must fail closed.
    let mut bad_sandbox = valid_capsule();
    bad_sandbox.sandbox_admitted = false;
    assert_eq!(
        admit_governed_context_capsule(bad_sandbox),
        Err(GovernedContextRejection::SandboxLethalTrifectaDenied)
    );

    // Invalid Cedar handoff must fail closed.
    let mut bad_cedar = valid_capsule();
    bad_cedar.cedar_handoff_valid = false;
    assert_eq!(
        admit_governed_context_capsule(bad_cedar),
        Err(GovernedContextRejection::InvalidCedarHandoff)
    );

    // BlockerActive detour with auto-archive still enabled violates Design 165 MarlinDetourBlockerProfile.
    let mut bad_blocker = valid_capsule();
    bad_blocker.detour_mode = DetourMode::BlockerActive;
    bad_blocker.auto_archive_enabled = true;
    assert_eq!(
        admit_governed_context_capsule(bad_blocker),
        Err(GovernedContextRejection::BlockerDetourMustDisableAutoArchive)
    );

    // BlockerActive detour with auto-archive disabled is admitted.
    let mut ok_blocker = valid_capsule();
    ok_blocker.detour_mode = DetourMode::BlockerActive;
    ok_blocker.auto_archive_enabled = false;
    assert!(admit_governed_context_capsule(ok_blocker).is_ok());
}
