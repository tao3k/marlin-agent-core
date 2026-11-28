// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM38 (ADV-L4) Defense Pillar Cryptographic & State-Machine Integrity Confinement tests.
//!
//! Exercises live exploits against B16, B18, B14, B20, B9, and B21 defense gates in
//! `marlin-agent-kernel`.

use std::collections::BTreeSet;

use marlin_agent_kernel::{
    BilateralTreaty, CryptoRewrapClaim, FederatedOpClaim, FederatedOrgReciprocityGate,
    FederatedOrgRejection, GovernanceProofDigest, GovernanceTransparencyLog, GovernanceTreeHead,
    KeyRotationShreddingGate, MerkleConsistencyProof, SupplyChainBuildClaim,
    SupplyChainDependencySpec, SupplyChainProvenanceGate, SupplyChainRejection,
    TransparencyLogRejection, ZkDisclosureClaim, ZkDisclosureRejection, ZkSelectiveDisclosureGate,
    compute_shard_merkle_root, governance_sha256_bytes, verify_tree_head_progression,
};

fn hex64(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn proof_digest(label: &str) -> GovernanceProofDigest {
    let raw = governance_sha256_bytes(label.as_bytes());
    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in raw {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{b:02x}");
    }
    GovernanceProofDigest::new(hex).expect("valid sha256 proof digest")
}

#[test]
fn sm38_b16_refuses_future_epoch_shred_dos_epoch0_rewrap_and_blank_tenant() {
    assert!(
        KeyRotationShreddingGate::new("   ", hex64('1')).is_err(),
        "blank tenant_id must fail closed"
    );

    let mut gate = KeyRotationShreddingGate::new("tenant-alpha", hex64('1')).expect("init gate");

    // Hostile exploit 1: tombstoning future epoch 2 before rotation must be refused
    // so rotation 1 -> 2 cannot be permanently bricked.
    assert!(
        gate.shred_retired_epoch(2, "future-dos").is_err(),
        "future epoch (> active_epoch) must never be shredded"
    );
    assert!(
        gate.shred_retired_epoch(0, "zero-epoch").is_err(),
        "non-existent epoch 0 must never be shredded"
    );
    assert!(
        !gate.is_epoch_shredded(2),
        "refused future-epoch shred must not insert tombstone"
    );

    // Rotation 1 -> 2 must still succeed cleanly.
    let rot = gate
        .rotate_to_epoch(2, hex64('2'))
        .expect("rotation 1->2 must not be bricked by prior refused future shred");
    assert_eq!(rot.active_epoch, 2);

    // Hostile exploit 2: rewrapping from non-existent epoch 0 must fail closed.
    let rewrap_zero = gate.admit_rewrap(&CryptoRewrapClaim {
        tenant_id: "tenant-alpha".to_owned(),
        from_epoch: 0,
        to_epoch: 2,
        before_plaintext_digest: hex64('a'),
        after_plaintext_digest: hex64('a'),
    });
    assert!(
        rewrap_zero.is_err(),
        "rewrap from non-existent epoch 0 must fail closed"
    );
}

#[test]
fn sm38_b18_refuses_case_whitespace_secret_leak_and_challenge_nonce_replay() {
    let mut gate = ZkSelectiveDisclosureGate::new(
        BTreeSet::from(["raw_pii".to_owned(), "signing_seed".to_owned()]),
        BTreeSet::from(["eu_kyc_valid".to_owned(), "sanctions_clear".to_owned()]),
    );
    gate.issue_challenge_nonce("zk-chal-01");

    let base_claim = ZkDisclosureClaim {
        proof_id: "zk-proof-01".to_owned(),
        commitment_root: hex64('9'),
        verifier_challenge_nonce: "zk-chal-01".to_owned(),
        nullifier_hex: hex64('d'),
        disclosed_fields: BTreeSet::from(["jurisdiction_tier".to_owned()]),
        proven_predicates: BTreeSet::from([
            "eu_kyc_valid".to_owned(),
            "sanctions_clear".to_owned(),
        ]),
        sigma_proof_verified: true,
    };

    // Hostile exploit 1: uppercase / whitespace-padded forbidden secrets must still be blocked.
    for variant in ["RAW_PII", " raw_pii ", "\tSigning_Seed\n"] {
        let mut leaky = base_claim.clone();
        leaky.disclosed_fields.insert(variant.to_owned());
        assert!(
            matches!(
                gate.admit_disclosure(&leaky),
                Err(ZkDisclosureRejection::SecretFieldOverDisclosed { .. })
            ),
            "forbidden secret variant {variant:?} must be refused"
        );
    }

    // Consume valid challenge once.
    gate.admit_disclosure(&base_claim)
        .expect("clean selective disclosure admits");

    // Hostile exploit 2: blank nonce and re-issued previously consumed nonce must fail closed.
    gate.issue_challenge_nonce("   ");
    let mut blank_nonce_claim = base_claim.clone();
    blank_nonce_claim.verifier_challenge_nonce = "   ".to_owned();
    blank_nonce_claim.nullifier_hex = hex64('e');
    assert!(
        matches!(
            gate.admit_disclosure(&blank_nonce_claim),
            Err(ZkDisclosureRejection::UnissuedChallengeNonce { .. })
        ),
        "blank verifier challenge nonce must be refused"
    );

    gate.issue_challenge_nonce("zk-chal-01");
    let mut replayed_nonce_claim = base_claim;
    replayed_nonce_claim.nullifier_hex = hex64('f');
    assert!(
        matches!(
            gate.admit_disclosure(&replayed_nonce_claim),
            Err(ZkDisclosureRejection::UnissuedChallengeNonce { .. })
        ),
        "re-issuing a previously consumed verifier challenge nonce must fail closed"
    );
}

#[test]
fn sm38_b14_refuses_case_whitespace_builder_spoofing_pin_overwrite_cve_pin_and_replay() {
    let mut gate = SupplyChainProvenanceGate::new();
    gate.register_pinned_dependency("marlin-agent-kernel", "0.4.0", hex64('a'))
        .expect("pin kernel");
    gate.register_pinned_dependency("ring", "0.17.14", hex64('b'))
        .expect("pin ring");

    // Hostile exploit 1: silent overwrite of an existing pin with a different digest must fail closed.
    assert!(
        gate.register_pinned_dependency("ring", "0.17.14", hex64('9'))
            .is_err(),
        "conflicting pin overwrite must be refused"
    );

    // Hostile exploit 2: pinning an already CVE-tombstoned coordinate must fail closed.
    gate.record_cve_tombstone("evil-pkg@1.0.0", "CVE-2026-42424");
    assert!(
        matches!(
            gate.register_pinned_dependency("EVIL-PKG", "1.0.0", hex64('c')),
            Err(SupplyChainRejection::CveTombstoneMatched { .. })
        ),
        "pinning a CVE-tombstoned package coordinate must fail closed"
    );

    let clean_claim = SupplyChainBuildClaim {
        release_id: "rel-2026.11.27".to_owned(),
        artifact_digest: hex64('f'),
        dependencies: vec![
            SupplyChainDependencySpec {
                package_name: "marlin-agent-kernel".to_owned(),
                version: "0.4.0".to_owned(),
                sha256_digest: hex64('a'),
            },
            SupplyChainDependencySpec {
                package_name: "ring".to_owned(),
                version: "0.17.14".to_owned(),
                sha256_digest: hex64('b'),
            },
        ],
        hermetic_sandbox: true,
        network_access_during_build: false,
        builder_a_id: "builder-eu-nix-01".to_owned(),
        builder_a_digest: hex64('f'),
        builder_b_id: "builder-us-nix-02".to_owned(),
        builder_b_digest: hex64('f'),
    };

    // Hostile exploit 3: case/whitespace-spoofed single builder bypassing 2-builder independence.
    let mut spoofed_builder = clean_claim.clone();
    spoofed_builder.builder_b_id = " BUILDER-EU-NIX-01 ".to_owned();
    assert!(
        matches!(
            gate.admit_build(&spoofed_builder),
            Err(SupplyChainRejection::BuilderParityFailed { .. })
        ),
        "case/whitespace builder_b_id matching builder_a_id must fail 2-builder parity"
    );

    // Hostile exploit 4: duplicate dependency entries inflating verified_dependency_count.
    let mut dup_dep = clean_claim.clone();
    dup_dep.dependencies.push(dup_dep.dependencies[1].clone());
    assert!(
        gate.admit_build(&dup_dep).is_err(),
        "duplicate dependency specs must be refused"
    );

    // Hostile exploit 5: replaying an already-admitted release_id must fail closed.
    gate.admit_build(&clean_claim)
        .expect("first clean build admission succeeds");
    assert!(
        gate.admit_build(&clean_claim).is_err(),
        "replaying an already-admitted release_id must fail closed"
    );
}

#[test]
fn sm38_b20_refuses_treaty_freeze_wipeout_epoch_rollback_and_self_reciprocity_case_spoof() {
    let mut gate = FederatedOrgReciprocityGate::new();

    // Hostile exploit 1: case/whitespace self-reciprocity spoofing in treaty registration.
    assert!(
        matches!(
            gate.register_treaty(BilateralTreaty {
                treaty_id: "treaty-self-spoof".to_owned(),
                org_a: "org-alpha".to_owned(),
                org_b: " ORG-ALPHA ".to_owned(),
                treaty_epoch: 1,
                ratified: true,
                expires_at_unix_ms: 1_900_000_000_000,
                dispute_freeze_active: false,
            }),
            Err(FederatedOrgRejection::SelfReciprocitySpoofing { .. })
        ),
        "case/whitespace self-reciprocity treaty must fail closed"
    );

    let treaty = BilateralTreaty {
        treaty_id: "treaty-alpha-beta".to_owned(),
        org_a: "org-alpha".to_owned(),
        org_b: "org-beta".to_owned(),
        treaty_epoch: 2,
        ratified: true,
        expires_at_unix_ms: 1_900_000_000_000,
        dispute_freeze_active: false,
    };
    gate.register_treaty(treaty.clone())
        .expect("register initial treaty");

    // Hostile exploit 2: case/whitespace self-reciprocity op claim.
    let self_op = FederatedOpClaim {
        op_id: "op-self-spoof".to_owned(),
        treaty_id: "treaty-alpha-beta".to_owned(),
        origin_org: "org-alpha".to_owned(),
        target_org: " ORG-ALPHA ".to_owned(),
        origin_signature_verified: true,
        target_signature_verified: true,
        observed_unix_ms: 1_700_000_000_000,
        payload_digest: hex64('7'),
    };
    assert!(
        matches!(
            gate.admit_federated_op(&self_op),
            Err(FederatedOrgRejection::SelfReciprocitySpoofing { .. })
        ),
        "case/whitespace self-reciprocity op must fail closed"
    );

    // Hostile exploit 3: re-registering treaty must NEVER wipe an active dispute freeze.
    gate.raise_dispute_freeze("treaty-alpha-beta")
        .expect("activate dispute freeze");
    assert!(
        matches!(
            gate.register_treaty(BilateralTreaty {
                treaty_epoch: 3,
                dispute_freeze_active: false,
                ..treaty.clone()
            }),
            Err(FederatedOrgRejection::DisputeFreezeActive { .. })
        ),
        "register_treaty must not clear an active dispute freeze without resolve_dispute_freeze"
    );

    // Treaty must still block federated ops while frozen.
    let valid_op = FederatedOpClaim {
        op_id: "op-clean-01".to_owned(),
        treaty_id: "treaty-alpha-beta".to_owned(),
        origin_org: "org-alpha".to_owned(),
        target_org: "org-beta".to_owned(),
        origin_signature_verified: true,
        target_signature_verified: true,
        observed_unix_ms: 1_700_000_000_000,
        payload_digest: hex64('7'),
    };
    assert!(matches!(
        gate.admit_federated_op(&valid_op),
        Err(FederatedOrgRejection::DisputeFreezeActive { .. })
    ));

    // Hostile exploit 4: rolling back treaty_epoch (2 -> 1) after resolution must fail closed.
    gate.resolve_dispute_freeze("treaty-alpha-beta")
        .expect("resolve freeze");
    assert!(
        gate.register_treaty(BilateralTreaty {
            treaty_epoch: 1,
            ..treaty
        })
        .is_err(),
        "register_treaty must refuse treaty_epoch rollback (2 -> 1)"
    );
}

#[test]
fn sm38_b9_b21_refuses_forged_empty_tree_head_size_mismatch_and_odd_leaf_merkle_malleability() {
    // B9 Hostile exploit 1: forged non-empty root hash at tree_size == 0 must fail closed.
    let mut log = GovernanceTransparencyLog::new();
    log.append(b"receipt-0".to_vec());
    log.append(b"receipt-1".to_vec());
    let head_2 = log.tree_head();
    let valid_zero_proof = log.consistency_proof(0).expect("proof from 0");

    let forged_empty_head = GovernanceTreeHead {
        tree_size: 0,
        root_hash: [0xaa; 32],
    };
    assert_eq!(
        verify_tree_head_progression(&forged_empty_head, &head_2, &valid_zero_proof),
        Err(TransparencyLogRejection::ConsistencyProofMismatch),
        "forged non-empty root hash at tree_size=0 must fail closed"
    );

    // B9 Hostile exploit 2: forged proof sizes on empty or equal tree-head transitions must fail closed.
    let valid_empty_head = GovernanceTransparencyLog::new().tree_head();
    let bad_sizes_proof = MerkleConsistencyProof {
        first_size: 99,
        second_size: 99,
        consistency_path: Vec::new(),
    };
    assert_eq!(
        verify_tree_head_progression(&valid_empty_head, &head_2, &bad_sizes_proof),
        Err(TransparencyLogRejection::ConsistencyProofMismatch),
        "mismatched proof first_size/second_size on empty-head progression must fail closed"
    );
    assert_eq!(
        verify_tree_head_progression(&head_2, &head_2, &bad_sizes_proof),
        Err(TransparencyLogRejection::ConsistencyProofMismatch),
        "mismatched proof first_size/second_size on equal-head progression must fail closed"
    );

    // B21 Hostile exploit 3: CVE-2012-2459 odd-leaf Merkle malleability ([A, B, C] == [A, B, C, C]).
    let s1 = proof_digest("tensor-shard-0001");
    let s2 = proof_digest("tensor-shard-0002");
    let s3 = proof_digest("tensor-shard-0003");
    let root_3 = compute_shard_merkle_root(&[s1.clone(), s2.clone(), s3.clone()])
        .expect("3-shard Merkle root");
    let root_4_dup = compute_shard_merkle_root(&[s1, s2, s3.clone(), s3])
        .expect("4-shard duplicated-tail Merkle root");
    assert_ne!(
        root_3, root_4_dup,
        "odd-leaf Merkle duplication must be eliminated: root([A,B,C]) != root([A,B,C,C])"
    );
}
