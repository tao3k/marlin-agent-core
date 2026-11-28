// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use marlin_agent_context::{
    ContextTaskId,
    governance::{
        AcquireRegionLeaseRequest, GovernanceRejectionReason, GovernedAnchorDescriptor,
        GovernedAnchorPropertyId, GovernedArchComponentId, GovernedEffectNonce, GovernedGeneration,
        GovernedLeaseId, GovernedOrgSnapshot, GovernedQualifiedAnchorId, GovernedSectionSpan,
        GovernedSourceId, GovernedSubtreeDigest, OrgGovernanceLedger, RegionLeaseReceipt,
        SubtreeCommitProposal, SubtreeCommitReceipt, SubtreeSemanticVerifier,
    },
};
use marlin_agent_protocol::{
    GraphQueryScoreBasisPoints, ProjectRuntimeAgentId, ProjectRuntimeProjectId,
    ProjectRuntimeSessionId, ProjectRuntimeWorktreeId,
};
use tokio::sync::{mpsc, oneshot};

struct DeterministicTestVerifier;

impl SubtreeSemanticVerifier for DeterministicTestVerifier {
    fn compute_subtree_digest(
        &self,
        property_id: &GovernedAnchorPropertyId,
        header: &str,
        body: &str,
    ) -> GovernedSubtreeDigest {
        let mut hasher = DefaultHasher::new();
        property_id.as_str().hash(&mut hasher);
        header.hash(&mut hasher);
        body.hash(&mut hasher);
        GovernedSubtreeDigest::new(format!("sha256:{:016x}", hasher.finish())).unwrap()
    }

    fn normalize_and_check_body(&self, _header: &str, replacement_body: &str) -> Option<String> {
        for line in replacement_body.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('*') || trimmed.eq_ignore_ascii_case(":PROPERTIES:") {
                return None;
            }
        }
        let mut out = replacement_body.to_owned();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        Some(out)
    }
}

enum CoordinatorCommand {
    AcquireLease {
        req: AcquireRegionLeaseRequest,
        reply: oneshot::Sender<RegionLeaseReceipt>,
    },
    CommitSubtree {
        proposal: SubtreeCommitProposal,
        reply: oneshot::Sender<SubtreeCommitReceipt>,
    },
    ReadSource {
        source_id: GovernedSourceId,
        reply: oneshot::Sender<(GovernedGeneration, String)>,
    },
}

fn build_shared_snapshot() -> GovernedOrgSnapshot {
    let verifier = DeterministicTestVerifier;
    let root_header = ":PROPERTIES:\n:ID: shared-root\n:CONTRACT_ID: context.governance.work.v1\n:CONTRACT_REVISION: 1\n:PROJECT_ID: marlin-agent-core\n:END:\n";
    let sec1_header = "* Spec Section\n:PROPERTIES:\n:ID: sec-spec\n:AGENT_ID: agent/architect-01\n:REVIEWER_AGENT_ID: agent/reviewer-09\n:TASK_ID: task/spec\n:END:\n";
    let sec1_body = "Initial spec.\n";
    let sec2_header = "* Runtime Section\n:PROPERTIES:\n:ID: sec-runtime\n:AGENT_ID: agent/rust-02\n:REVIEWER_AGENT_ID: agent/architect-01\n:TASK_ID: task/runtime\n:END:\n";
    let sec2_body = "Initial runtime.\n";
    let sec3_header = "* Verification Section\n:PROPERTIES:\n:ID: sec-verify\n:AGENT_ID: agent/verifier-03\n:REVIEWER_AGENT_ID: agent/architect-01\n:TASK_ID: task/verify\n:END:\n";
    let sec3_body = "Initial verification.\n";

    let full = format!(
        "{root_header}{sec1_header}{sec1_body}{sec2_header}{sec2_body}{sec3_header}{sec3_body}"
    );

    let s1_start = root_header.len();
    let s1_drawer_end = s1_start + sec1_header.len();
    let s1_end = s1_drawer_end + sec1_body.len();

    let s2_start = s1_end;
    let s2_drawer_end = s2_start + sec2_header.len();
    let s2_end = s2_drawer_end + sec2_body.len();

    let s3_start = s2_end;
    let s3_drawer_end = s3_start + sec3_header.len();
    let s3_end = s3_drawer_end + sec3_body.len();

    let mk_desc = |local_id: &str,
                   agent: &str,
                   reviewer: &str,
                   task: &str,
                   c_start: usize,
                   d_end: usize,
                   c_end: usize,
                   hdr: &str,
                   bdy: &str| {
        let prop_id = GovernedAnchorPropertyId::new(local_id).unwrap();
        let dig = verifier.compute_subtree_digest(&prop_id, hdr, bdy);
        (
            prop_id.clone(),
            GovernedAnchorDescriptor {
                qualified_id: GovernedQualifiedAnchorId::new(format!("sha256:q-{local_id}"))
                    .unwrap(),
                property_id: prop_id,
                project_id: ProjectRuntimeProjectId::new("marlin-agent-core"),
                worktree_id: Some(ProjectRuntimeWorktreeId::new("main")),
                component_id: Some(GovernedArchComponentId::new("marlin-deck-runtime").unwrap()),
                owner_agent_id: Some(ProjectRuntimeAgentId::new(agent)),
                reviewer_agent_id: Some(ProjectRuntimeAgentId::new(reviewer)),
                task_id: Some(ContextTaskId::new(task)),
                span: GovernedSectionSpan::new(c_start, d_end, c_end, c_end).unwrap(),
                subtree_digest: dig,
                affinity_score: GraphQueryScoreBasisPoints::new(10_000),
            },
        )
    };

    let mut anchors = HashMap::new();
    let (k1, v1) = mk_desc(
        "sec-spec",
        "agent/architect-01",
        "agent/reviewer-09",
        "task/spec",
        s1_start,
        s1_drawer_end,
        s1_end,
        sec1_header,
        sec1_body,
    );
    let (k2, v2) = mk_desc(
        "sec-runtime",
        "agent/rust-02",
        "agent/architect-01",
        "task/runtime",
        s2_start,
        s2_drawer_end,
        s2_end,
        sec2_header,
        sec2_body,
    );
    let (k3, v3) = mk_desc(
        "sec-verify",
        "agent/verifier-03",
        "agent/architect-01",
        "task/verify",
        s3_start,
        s3_drawer_end,
        s3_end,
        sec3_header,
        sec3_body,
    );
    anchors.insert(k1, v1);
    anchors.insert(k2, v2);
    anchors.insert(k3, v3);

    GovernedOrgSnapshot {
        source_id: GovernedSourceId::new("shared-context.org").unwrap(),
        generation: GovernedGeneration::new(1).unwrap(),
        source: full,
        anchors,
    }
}

#[tokio::test]
async fn three_tokio_agents_concurrently_lease_and_commit_disjoint_org_ids_without_conflict() {
    let (tx, mut rx) = mpsc::channel::<CoordinatorCommand>(32);

    // Single-owner coordinator task (mirrors the Deck single-VM owner thread).
    let owner_handle = tokio::spawn(async move {
        let mut ledger = OrgGovernanceLedger::new(DeterministicTestVerifier);
        ledger.register_snapshot(build_shared_snapshot()).unwrap();
        while let Some(cmd) = rx.recv().await {
            match cmd {
                CoordinatorCommand::AcquireLease { req, reply } => {
                    let _ = reply.send(ledger.acquire_lease(req));
                }
                CoordinatorCommand::CommitSubtree { proposal, reply } => {
                    let _ = reply.send(ledger.commit_subtree(proposal).unwrap());
                }
                CoordinatorCommand::ReadSource { source_id, reply } => {
                    let snap = ledger.snapshot(&source_id).unwrap();
                    let _ = reply.send((snap.generation, snap.source.clone()));
                }
            }
        }
    });

    let spawn_agent = |tx: mpsc::Sender<CoordinatorCommand>,
                       lease_name: &'static str,
                       nonce_name: &'static str,
                       anchor: &'static str,
                       agent: &'static str,
                       task: &'static str,
                       session: &'static str,
                       body: &'static str| {
        tokio::spawn(async move {
            let source_id = GovernedSourceId::new("shared-context.org").unwrap();
            let target_property_id = GovernedAnchorPropertyId::new(anchor).unwrap();
            let lease_id = GovernedLeaseId::new(lease_name).unwrap();
            let agent_id = ProjectRuntimeAgentId::new(agent);
            let task_id = ContextTaskId::new(task);
            let session_id = ProjectRuntimeSessionId::new(session);

            let (l_tx, l_rx) = oneshot::channel();
            tx.send(CoordinatorCommand::AcquireLease {
                req: AcquireRegionLeaseRequest {
                    lease_id: lease_id.clone(),
                    source_id: source_id.clone(),
                    target_property_id: target_property_id.clone(),
                    agent_id: agent_id.clone(),
                    task_id: task_id.clone(),
                    session_id: session_id.clone(),
                },
                reply: l_tx,
            })
            .await
            .unwrap();

            let RegionLeaseReceipt::Granted(granted) = l_rx.await.unwrap() else {
                panic!("disjoint agent lease must be granted");
            };

            let (c_tx, c_rx) = oneshot::channel();
            tx.send(CoordinatorCommand::CommitSubtree {
                proposal: SubtreeCommitProposal {
                    lease_id,
                    nonce: GovernedEffectNonce::new(nonce_name).unwrap(),
                    source_id,
                    base_generation: granted.acquired_at_generation,
                    target_property_id,
                    agent_id,
                    task_id,
                    session_id,
                    expected_subtree_digest: granted.expected_subtree_digest,
                    replacement_body: body.to_owned(),
                },
                reply: c_tx,
            })
            .await
            .unwrap();

            c_rx.await.unwrap()
        })
    };

    let a1 = spawn_agent(
        tx.clone(),
        "lease-1",
        "nonce-1",
        "sec-spec",
        "agent/architect-01",
        "task/spec",
        "sess-1",
        "Detailed UTF-8 架构规范 with extra lines shifting byte offsets!\nLine B\n",
    );
    let a2 = spawn_agent(
        tx.clone(),
        "lease-2",
        "nonce-2",
        "sec-runtime",
        "agent/rust-02",
        "task/runtime",
        "sess-2",
        "Async Tokio mpsc + Deck VM lease table implementation.\n",
    );
    let a3 = spawn_agent(
        tx.clone(),
        "lease-3",
        "nonce-3",
        "sec-verify",
        "agent/verifier-03",
        "task/verify",
        "sess-3",
        "All Quint invariants and Scheme/Rust gates verified.\n",
    );

    let (r1, r2, r3) = tokio::join!(a1, a2, a3);
    let (r1, r2, r3) = (r1.unwrap(), r2.unwrap(), r3.unwrap());
    assert!(r1.accepted && r2.accepted && r3.accepted);

    let mut gens = [
        r1.committed_generation.unwrap().get(),
        r2.committed_generation.unwrap().get(),
        r3.committed_generation.unwrap().get(),
    ];
    gens.sort_unstable();
    assert_eq!(gens, [2, 3, 4]);

    let (s_tx, s_rx) = oneshot::channel();
    tx.send(CoordinatorCommand::ReadSource {
        source_id: GovernedSourceId::new("shared-context.org").unwrap(),
        reply: s_tx,
    })
    .await
    .unwrap();
    let (final_gen, final_source) = s_rx.await.unwrap();
    assert_eq!(final_gen.get(), 4);
    assert!(final_source.contains("Detailed UTF-8 架构规范"));
    assert!(final_source.contains("Async Tokio mpsc + Deck VM lease table"));
    assert!(final_source.contains("All Quint invariants and Scheme/Rust gates verified."));
    // Governance property drawers remain intact!
    assert!(final_source.contains(":AGENT_ID: agent/architect-01"));
    assert!(final_source.contains(":AGENT_ID: agent/rust-02"));
    assert!(final_source.contains(":AGENT_ID: agent/verifier-03"));

    drop(tx);
    owner_handle.await.unwrap();
}

#[test]
fn overlapping_lease_cross_agent_hijack_and_header_escape_are_rejected_with_typed_receipts() {
    let mut ledger = OrgGovernanceLedger::new(DeterministicTestVerifier);
    ledger.register_snapshot(build_shared_snapshot()).unwrap();
    let source_id = GovernedSourceId::new("shared-context.org").unwrap();

    // 1. Wrong agent attempting to lease `sec-spec` is denied (`WriteIneligibleForAgentOrTask`).
    let hijack = ledger.acquire_lease(AcquireRegionLeaseRequest {
        lease_id: GovernedLeaseId::new("lease-hijack").unwrap(),
        source_id: source_id.clone(),
        target_property_id: GovernedAnchorPropertyId::new("sec-spec").unwrap(),
        agent_id: ProjectRuntimeAgentId::new("agent/rust-02"),
        task_id: ContextTaskId::new("task/spec"),
        session_id: ProjectRuntimeSessionId::new("sess-bad"),
    });
    assert!(matches!(
        hijack,
        RegionLeaseReceipt::Denied {
            reason: GovernanceRejectionReason::WriteIneligibleForAgentOrTask,
            ..
        }
    ));

    // 2. Legitimate owner acquires `sec-spec`, then a second lease on the same `:ID:` is denied (`OverlappingRegionLease`).
    let granted = ledger.acquire_lease(AcquireRegionLeaseRequest {
        lease_id: GovernedLeaseId::new("lease-ok").unwrap(),
        source_id: source_id.clone(),
        target_property_id: GovernedAnchorPropertyId::new("sec-spec").unwrap(),
        agent_id: ProjectRuntimeAgentId::new("agent/architect-01"),
        task_id: ContextTaskId::new("task/spec"),
        session_id: ProjectRuntimeSessionId::new("sess-1"),
    });
    let RegionLeaseReceipt::Granted(active) = granted else {
        panic!("expected lease grant");
    };

    let overlap = ledger.acquire_lease(AcquireRegionLeaseRequest {
        lease_id: GovernedLeaseId::new("lease-overlap").unwrap(),
        source_id: source_id.clone(),
        target_property_id: GovernedAnchorPropertyId::new("sec-spec").unwrap(),
        agent_id: ProjectRuntimeAgentId::new("agent/architect-01"),
        task_id: ContextTaskId::new("task/spec"),
        session_id: ProjectRuntimeSessionId::new("sess-2"),
    });
    assert!(matches!(
        overlap,
        RegionLeaseReceipt::Denied {
            reason: GovernanceRejectionReason::OverlappingRegionLease { .. },
            ..
        }
    ));

    // 3. Attempting to smuggle a `* Rogue Headline` / `:PROPERTIES:` drawer inside body is rejected!
    let escape = ledger
        .commit_subtree(SubtreeCommitProposal {
            lease_id: active.lease_id.clone(),
            nonce: GovernedEffectNonce::new("nonce-escape").unwrap(),
            source_id,
            base_generation: active.acquired_at_generation,
            target_property_id: active.target_property_id,
            agent_id: active.agent_id,
            task_id: active.task_id,
            session_id: active.session_id,
            expected_subtree_digest: active.expected_subtree_digest,
            replacement_body: "Safe line\n* Injected Headline\n:PROPERTIES:\n:ID: evil\n:END:\n"
                .to_owned(),
        })
        .unwrap();
    assert!(!escape.accepted);
    assert_eq!(
        escape.rejection,
        Some(GovernanceRejectionReason::HeaderOrStructuralEscape)
    );
}
