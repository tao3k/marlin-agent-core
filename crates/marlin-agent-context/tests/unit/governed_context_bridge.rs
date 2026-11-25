// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::HashMap;

use marlin_agent_context::ContextTaskId;
use marlin_agent_context::governance::{
    AcquireRegionLeaseRequest, GovernedAnchorDescriptor, GovernedAnchorPropertyId,
    GovernedEffectNonce, GovernedGeneration, GovernedLeaseId, GovernedOrgSnapshot,
    GovernedQualifiedAnchorId, GovernedSectionSpan, GovernedSourceId, GovernedSubtreeDigest,
    OrgGovernanceLedger, RegionLeaseReceipt, SubtreeCommitProposal, SubtreeSemanticVerifier,
};
use marlin_agent_context::governed_bridge::{
    GovernedContextBridgeError, TwoZonePromptEnvelope, assemble_two_zone_gateway_messages,
    commit_governed_org_subtree, compute_kv_prefix_digest,
};
use marlin_agent_graph::{
    DetourMode, DetourNodeId, DetourState, DisclosureDepth, FencingEpoch, GovernedContextCapsule,
    TaskActorCursor, TaskActorId, TaskActorScopeMode, TaskActorStatus,
    admit_governed_context_capsule,
};
use marlin_agent_protocol::{
    GraphQueryScoreBasisPoints, ModelGatewayMessageRole, ProjectRuntimeAgentId,
    ProjectRuntimeProjectId, ProjectRuntimeSessionId,
};
use sha2::{Digest, Sha256};

struct DeterministicVerifier;

impl SubtreeSemanticVerifier for DeterministicVerifier {
    fn compute_subtree_digest(
        &self,
        property_id: &GovernedAnchorPropertyId,
        header: &str,
        body: &str,
    ) -> GovernedSubtreeDigest {
        let mut hasher = Sha256::new();
        hasher.update(property_id.as_str().as_bytes());
        hasher.update(header.as_bytes());
        hasher.update(body.as_bytes());
        GovernedSubtreeDigest::new(format!("sha256:{:x}", hasher.finalize())).unwrap()
    }

    fn normalize_and_check_body(&self, _header: &str, replacement_body: &str) -> Option<String> {
        if replacement_body.contains("\n* ") || replacement_body.contains(":PROPERTIES:") {
            None
        } else {
            Some(replacement_body.to_owned())
        }
    }
}

#[test]
fn org_governance_ledger_and_governed_context_capsule_enforce_detour_and_two_zone_kv_prefix() {
    let source = "* TODO Mission\n:PROPERTIES:\n:ID: sec-1\n:END:\ninitial body\n".to_owned();
    let drawer_end = source.find("initial body").unwrap();
    let span = GovernedSectionSpan::new(0, drawer_end, source.len(), source.len()).unwrap();
    let prop_id = GovernedAnchorPropertyId::new("sec-1").unwrap();
    let verifier = DeterministicVerifier;
    let initial_digest =
        verifier.compute_subtree_digest(&prop_id, &source[0..drawer_end], &source[drawer_end..]);

    let agent_id = ProjectRuntimeAgentId::new("agent-alpha");
    let ctx_task_id = ContextTaskId::new("task-ctx-01");
    let session_id = ProjectRuntimeSessionId::new("sess-1");
    let source_id = GovernedSourceId::new("context.org").unwrap();

    let mut anchors = HashMap::new();
    anchors.insert(
        prop_id.clone(),
        GovernedAnchorDescriptor {
            qualified_id: GovernedQualifiedAnchorId::new("sha256:q-sec-1").unwrap(),
            property_id: prop_id.clone(),
            project_id: ProjectRuntimeProjectId::new("marlin-agent-core"),
            worktree_id: None,
            component_id: None,
            span,
            subtree_digest: initial_digest.clone(),
            owner_agent_id: Some(agent_id.clone()),
            reviewer_agent_id: None,
            task_id: Some(ctx_task_id.clone()),
            affinity_score: GraphQueryScoreBasisPoints::new(10_000),
        },
    );

    let mut ledger = OrgGovernanceLedger::new(verifier);
    ledger
        .register_snapshot(GovernedOrgSnapshot {
            source_id: source_id.clone(),
            generation: GovernedGeneration::new(1).unwrap(),
            source,
            anchors,
        })
        .unwrap();

    let lease_id = GovernedLeaseId::new("lease-1").unwrap();
    let granted = ledger.acquire_lease(AcquireRegionLeaseRequest {
        lease_id: lease_id.clone(),
        source_id: source_id.clone(),
        target_property_id: prop_id.clone(),
        agent_id: agent_id.clone(),
        task_id: ctx_task_id.clone(),
        session_id: session_id.clone(),
    });
    assert!(matches!(granted, RegionLeaseReceipt::Granted(_)));

    let task_id = TaskActorId::new("task-ctx-01").unwrap();
    let cursor = TaskActorCursor {
        task_id: task_id.clone(),
        fencing_epoch: FencingEpoch::new(1).unwrap(),
        status: TaskActorStatus::Running,
        scope_mode: TaskActorScopeMode::Project,
        pending_steering_branches: 0,
    };
    let frozen_prefix = "SYSTEM: Governed Org Trunk v1";
    let admitted = admit_governed_context_capsule(GovernedContextCapsule {
        task_id: task_id.clone(),
        context_cut: 1,
        kv_prefix_digest: compute_kv_prefix_digest(frozen_prefix),
        disclosure_depth: DisclosureDepth::L3,
        has_active_defeater: true,
        quarantined_memory_count: 2,
        sandbox_admitted: true,
        cedar_handoff_valid: true,
        detour_mode: DetourMode::Mainline,
        auto_archive_enabled: true,
    })
    .unwrap();

    let make_proposal = |nonce: &str| SubtreeCommitProposal {
        lease_id: lease_id.clone(),
        nonce: GovernedEffectNonce::new(nonce).unwrap(),
        source_id: source_id.clone(),
        base_generation: GovernedGeneration::new(1).unwrap(),
        target_property_id: prop_id.clone(),
        agent_id: agent_id.clone(),
        task_id: ctx_task_id.clone(),
        session_id: session_id.clone(),
        expected_subtree_digest: initial_digest.clone(),
        replacement_body: "verified evidence body\n".to_owned(),
    };

    // 1. BlockerActive Scoped Detour halts Org trunk commit fail-closed!
    let blocker = DetourState::Active {
        mode: DetourMode::BlockerActive,
        origin_epoch: FencingEpoch::new(1).unwrap(),
        detour_node_id: DetourNodeId::new("detour-blk-1").unwrap(),
    };
    let blocked = commit_governed_org_subtree(
        &mut ledger,
        make_proposal("nonce-blocked"),
        &cursor,
        &blocker,
        &admitted,
    );
    assert_eq!(
        blocked,
        Err(GovernedContextBridgeError::BlockerDetourBlocksOrgCommit {
            task_id: "task-ctx-01".to_owned(),
        })
    );

    // 2. Mainline commit succeeds, advances context_cut 1 -> 2, and updates kv_prefix_digest!
    let committed = commit_governed_org_subtree(
        &mut ledger,
        make_proposal("nonce-ok-1"),
        &cursor,
        &DetourState::Mainline,
        &admitted,
    )
    .unwrap();
    assert!(committed.org_receipt.accepted);
    let next_ctx = committed.next_context.expect("next governed context");
    assert_eq!(next_ctx.capsule().context_cut, 2);
    assert_ne!(
        next_ctx.capsule().kv_prefix_digest,
        admitted.capsule().kv_prefix_digest
    );

    // 3. Two-Zone KV-Prefix prompt assembly enforces exact digest & L3 defeater floor!
    let env_ok = TwoZonePromptEnvelope::from_frozen_and_tail(
        frozen_prefix,
        DisclosureDepth::L3,
        "Run targeted verification",
    );
    let messages = assemble_two_zone_gateway_messages(&env_ok, &admitted, &[]).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, ModelGatewayMessageRole::System);
    assert!(messages[0].content.contains("kv_prefix=sha256:"));
    assert!(messages[0].content.contains("quarantined=2"));
    assert_eq!(messages[1].role, ModelGatewayMessageRole::User);

    // Tampering with frozen prefix breaks KV-Prefix digest equality fail-closed!
    let env_tampered = TwoZonePromptEnvelope::from_frozen_and_tail(
        "SYSTEM: Mutated Prefix",
        DisclosureDepth::L3,
        "Run targeted verification",
    );
    assert!(matches!(
        assemble_two_zone_gateway_messages(&env_tampered, &admitted, &[]),
        Err(GovernedContextBridgeError::KvPrefixDigestMismatch { .. })
    ));

    // Lowering disclosure below L3 under active defeater fails closed!
    let env_low_depth = TwoZonePromptEnvelope::from_frozen_and_tail(
        frozen_prefix,
        DisclosureDepth::L1,
        "Run targeted verification",
    );
    assert_eq!(
        assemble_two_zone_gateway_messages(&env_low_depth, &admitted, &[]),
        Err(GovernedContextBridgeError::DisclosureFloorViolation {
            requested: DisclosureDepth::L1,
            admitted: DisclosureDepth::L3,
        })
    );
}
