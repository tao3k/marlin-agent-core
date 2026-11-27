// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! ADV-L4 Hostile Adversarial & Cross-Pillar Chaos Integration Suite for the
//! 11-Pillar `UnifiedDefensePillarCoordinator` (`B9` + `B12`..`B21`).

use std::collections::BTreeSet;

use marlin_agent_kernel::{
    ArbitrationClaim, ArbitrationPhase, BilateralTreaty, ConfidentialEnclaveGate,
    ContextDistillationGate, ContextSegment, CryptoSuite, CryptoWriteClaim, DistilledContextPacket,
    EnclaveAttestationQuote, EpistemicIntegrity, FederatedArbitrationLivenessGate,
    FederatedOpClaim, FederatedOrgReciprocityGate, GeoPlacementClaim, GeoResidencySovereigntyGate,
    GovernanceProofDigest, GovernanceTransparencyLog, GovernedSubjectName, HybridSignatureEnvelope,
    KeyRotationShreddingGate, ModelInferenceClaim, ModelWeightAttestationGate,
    ModelWeightGovernancePolicy, PostQuantumAgilityGate, SecrecyClass, SupplyChainBuildClaim,
    SupplyChainDependencySpec, SupplyChainProvenanceGate, UnifiedDefensePillarCoordinator,
    UnifiedDefenseRejection, UnifiedDefenseReleaseClaim, ZkDisclosureClaim,
    ZkSelectiveDisclosureGate, compute_shard_merkle_root, verify_inclusion_proof,
    verify_tree_head_progression,
};

fn hex64_from_seed(seed: u64) -> String {
    let mut out = String::with_capacity(64);
    let mut x = seed;
    for _ in 0..4 {
        x = splitmix64(x);
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{x:016x}");
    }
    out
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn hex64(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn proof_digest(ch: char) -> GovernanceProofDigest {
    GovernanceProofDigest::new(format!("sha256:{}", hex64(ch))).expect("valid proof digest")
}

fn subject(name: &str) -> GovernedSubjectName {
    GovernedSubjectName::new(name).expect("valid governed subject")
}

fn build_clean_fixture() -> (UnifiedDefensePillarCoordinator, UnifiedDefenseReleaseClaim) {
    let b13 = FederatedArbitrationLivenessGate::new(2).expect("init b13");

    let mut b14 = SupplyChainProvenanceGate::new();
    b14.register_pinned_dependency("marlin-agent-kernel", "0.4.0", hex64('a'))
        .expect("pin b14");

    let mut b15 = ConfidentialEnclaveGate::new(14);
    b15.register_measurement(hex64('e'), Some(hex64('f')))
        .expect("pin b15");
    b15.issue_challenge_nonce("nonce-adv-00");

    let b16 = KeyRotationShreddingGate::new("tenant-eu", hex64('1')).expect("init b16");
    let b17 = PostQuantumAgilityGate::default();

    let mut b18 = ZkSelectiveDisclosureGate::new(
        BTreeSet::from(["raw_pii".to_owned(), "signing_seed".to_owned()]),
        BTreeSet::from(["eu_kyc_valid".to_owned(), "sanctions_clear".to_owned()]),
    );
    b18.issue_challenge_nonce("zk-adv-00");

    let mut b19 = GeoResidencySovereigntyGate::new();
    b19.allow_storage_region("EU-DE");
    b19.allow_inference_region("EU-DE");
    b19.trust_authority("eu-dpa-1");

    let mut b20 = FederatedOrgReciprocityGate::new();
    b20.register_treaty(BilateralTreaty {
        treaty_id: "treaty-eu-adv".to_owned(),
        org_a: "org-alpha".to_owned(),
        org_b: "org-beta".to_owned(),
        treaty_epoch: 1,
        ratified: true,
        expires_at_unix_ms: 1_900_000_000_000,
        dispute_freeze_active: false,
    })
    .expect("treaty b20");

    let shards = vec![proof_digest('1'), proof_digest('2')];
    let base_root = compute_shard_merkle_root(&shards).expect("merkle root");
    let kernel_digest = proof_digest('8');
    let b21 = ModelWeightAttestationGate::new(ModelWeightGovernancePolicy {
        model_id: subject("marlin-sovereign-7b"),
        pinned_weight_root: base_root.clone(),
        allowlisted_adapters: BTreeSet::new(),
        pinned_serving_kernel_digest: kernel_digest.clone(),
        require_deterministic_sampling: true,
        max_temperature_milli: 0,
        revoked_models: BTreeSet::new(),
    });

    let coordinator = UnifiedDefensePillarCoordinator {
        b9_transparency_log: GovernanceTransparencyLog::new(),
        b12_context_distillation: ContextDistillationGate::new(4096),
        b13_federated_arbitration: b13,
        b14_supply_chain: b14,
        b15_confidential_enclave: b15,
        b16_key_rotation: b16,
        b17_post_quantum: b17,
        b18_zk_disclosure: b18,
        b19_geo_residency: b19,
        b20_federated_org: b20,
        b21_model_weight: b21,
    };

    let seg_id = subject("seg-policy");
    let seg_digest = proof_digest('7');
    let claim = UnifiedDefenseReleaseClaim {
        release_id: "rel-adv-00".to_owned(),
        b12_source_segments: vec![ContextSegment {
            segment_id: seg_id.clone(),
            content_digest: seg_digest.clone(),
            integrity: EpistemicIntegrity::PinnedM1,
            secrecy: SecrecyClass::Internal,
            token_count: 120,
            is_pinned_constraint: true,
        }],
        b12_distilled_packet: DistilledContextPacket {
            cited_sources: vec![(seg_id.clone(), seg_digest)],
            retained_pinned_ids: BTreeSet::from([seg_id]),
            output_integrity: EpistemicIntegrity::PinnedM1,
            output_secrecy: SecrecyClass::Internal,
            output_tokens: 64,
            summary_digest: proof_digest('6'),
        },
        b13_arbitration: ArbitrationClaim {
            case_id: "arb-adv-00".to_owned(),
            wait_for_edges: vec![("org-alpha".to_owned(), "org-beta".to_owned())],
        },
        b14_supply_chain: SupplyChainBuildClaim {
            release_id: "rel-adv-00".to_owned(),
            artifact_digest: hex64('f'),
            dependencies: vec![SupplyChainDependencySpec {
                package_name: "marlin-agent-kernel".to_owned(),
                version: "0.4.0".to_owned(),
                sha256_digest: hex64('a'),
            }],
            hermetic_sandbox: true,
            network_access_during_build: false,
            builder_a_id: "builder-eu-1".to_owned(),
            builder_a_digest: hex64('f'),
            builder_b_id: "builder-eu-2".to_owned(),
            builder_b_digest: hex64('f'),
        },
        b15_enclave_quote: EnclaveAttestationQuote {
            enclave_id: "tdx-eu-01".to_owned(),
            debug_mode_enabled: false,
            measurement_digest: hex64('e'),
            tcb_svn: 15,
            challenge_nonce: "nonce-adv-00".to_owned(),
            bound_b14_artifact_digest: Some(hex64('f')),
            quote_signature_verified: true,
        },
        b16_crypto_write: CryptoWriteClaim {
            tenant_id: "tenant-eu".to_owned(),
            write_epoch: 1,
            plaintext_digest: hex64('c'),
        },
        b17_hybrid_envelope: HybridSignatureEnvelope {
            envelope_id: "env-adv-00".to_owned(),
            suite: CryptoSuite::HybridEd25519MlDsa65,
            domain_tag: "marlin.unified.release.v2".to_owned(),
            payload_digest: hex64('f'),
            classical_leg_verified: true,
            pq_leg_verified: true,
            ots_leaf_index: 1,
        },
        b18_zk_disclosure: ZkDisclosureClaim {
            proof_id: "zk-adv-00".to_owned(),
            commitment_root: hex64('9'),
            verifier_challenge_nonce: "zk-adv-00".to_owned(),
            nullifier_hex: hex64('d'),
            disclosed_fields: BTreeSet::from(["jurisdiction_tier".to_owned()]),
            proven_predicates: BTreeSet::from([
                "eu_kyc_valid".to_owned(),
                "sanctions_clear".to_owned(),
            ]),
            sigma_proof_verified: true,
        },
        b19_geo_placement: GeoPlacementClaim {
            placement_id: "geo-adv-00".to_owned(),
            tenant_id: "tenant-eu".to_owned(),
            source_region: "EU-DE".to_owned(),
            target_storage_region: "EU-DE".to_owned(),
            inference_region: "EU-DE".to_owned(),
            observed_unix_ms: 1_700_000_000_000,
            cross_border_permit: None,
        },
        b20_federated_op: FederatedOpClaim {
            op_id: "fed-adv-00".to_owned(),
            treaty_id: "treaty-eu-adv".to_owned(),
            origin_org: "org-alpha".to_owned(),
            target_org: "org-beta".to_owned(),
            origin_signature_verified: true,
            target_signature_verified: true,
            observed_unix_ms: 1_700_000_000_000,
            payload_digest: hex64('f'),
        },
        b21_model_inference: ModelInferenceClaim {
            model_id: subject("marlin-sovereign-7b"),
            observed_weight_root: base_root,
            active_adapters: Vec::new(),
            observed_serving_kernel_digest: kernel_digest,
            temperature_milli: 0,
            prng_seed: Some(42),
        },
    };

    (coordinator, claim)
}

#[test]
fn adv_l4_all_10_single_pillar_sabotages_rollback_atomically() {
    let (mut coord, clean_claim) = build_clean_fixture();

    for pillar_idx in 0..10 {
        let mut claim = clean_claim.clone();
        let err = match pillar_idx {
            0 => {
                claim.b12_distilled_packet.retained_pinned_ids.clear();
                coord.admit_release(&claim).unwrap_err()
            }
            1 => {
                claim.b13_arbitration.wait_for_edges = vec![
                    ("org-alpha".to_owned(), "org-beta".to_owned()),
                    ("org-beta".to_owned(), "org-alpha".to_owned()),
                ];
                coord.admit_release(&claim).unwrap_err()
            }
            2 => {
                claim.b14_supply_chain.network_access_during_build = true;
                coord.admit_release(&claim).unwrap_err()
            }
            3 => {
                claim.b15_enclave_quote.debug_mode_enabled = true;
                coord.admit_release(&claim).unwrap_err()
            }
            4 => {
                claim.b16_crypto_write.write_epoch = 99;
                coord.admit_release(&claim).unwrap_err()
            }
            5 => {
                claim.b17_hybrid_envelope.pq_leg_verified = false;
                coord.admit_release(&claim).unwrap_err()
            }
            6 => {
                claim
                    .b18_zk_disclosure
                    .disclosed_fields
                    .insert("raw_pii".to_owned());
                coord.admit_release(&claim).unwrap_err()
            }
            7 => {
                claim.b19_geo_placement.inference_region = "US-EAST".to_owned();
                coord.admit_release(&claim).unwrap_err()
            }
            8 => {
                claim.b20_federated_op.target_signature_verified = false;
                coord.admit_release(&claim).unwrap_err()
            }
            9 => {
                claim.b21_model_inference.temperature_milli = 700;
                coord.admit_release(&claim).unwrap_err()
            }
            _ => unreachable!(),
        };

        match (pillar_idx, &err) {
            (0, UnifiedDefenseRejection::B12ContextDistillation(_))
            | (1, UnifiedDefenseRejection::B13FederatedArbitration(_))
            | (2, UnifiedDefenseRejection::B14SupplyChain(_))
            | (3, UnifiedDefenseRejection::B15ConfidentialEnclave(_))
            | (4, UnifiedDefenseRejection::B16KeyRotation(_))
            | (5, UnifiedDefenseRejection::B17PostQuantum(_))
            | (6, UnifiedDefenseRejection::B18ZkSelectiveDisclosure(_))
            | (7, UnifiedDefenseRejection::B19GeoResidency(_))
            | (8, UnifiedDefenseRejection::B20FederatedOrg(_))
            | (9, UnifiedDefenseRejection::B21ModelWeight(_)) => {}
            other => panic!("unexpected rejection mapping: {other:?}"),
        }

        // Strict transactional rollback invariant across EVERY stateful gate & B9 log.
        assert_eq!(coord.b9_transparency_log.tree_head().tree_size, 0);
        assert_eq!(
            coord.b13_federated_arbitration.phase_of("arb-adv-00"),
            ArbitrationPhase::Active
        );
        assert_eq!(coord.b15_confidential_enclave.keys_released(), 0);
        assert!(!coord.b18_zk_disclosure.is_nullifier_spent(&hex64('d')));
        assert_eq!(coord.b20_federated_org.admitted_ops(), 0);
    }

    // After 10 consecutive single-pillar attacks, the unmodified clean claim still passes!
    let cert = coord
        .admit_release(&clean_claim)
        .expect("clean claim succeeds after 10 rollbacks");
    assert_eq!(cert.b9_tree_head.tree_size, 1);
    let proof = coord
        .b9_transparency_log
        .inclusion_proof(cert.b9_log_leaf_index)
        .expect("inclusion proof");
    verify_inclusion_proof(cert.composite_digest.as_bytes(), &proof, &cert.b9_tree_head)
        .expect("RFC 6962 inclusion proof verifies");
}

#[test]
fn adv_l4_multi_release_b9_consistency_and_replay_resistance() {
    let (mut coord, base_claim) = build_clean_fixture();
    let mut tree_heads = Vec::new();
    let mut digests = Vec::new();

    for i in 0..8u64 {
        let mut claim = base_claim.clone();
        claim.release_id = format!("rel-seq-{i}");
        claim.b13_arbitration.case_id = format!("arb-seq-{i}");
        claim.b14_supply_chain.release_id = format!("rel-seq-{i}");
        let enclave_nonce = format!("nonce-seq-{i}");
        coord
            .b15_confidential_enclave
            .issue_challenge_nonce(enclave_nonce.clone());
        claim.b15_enclave_quote.challenge_nonce = enclave_nonce;

        if i == 4 {
            // Rotate B16 active epoch 1 -> 2 and crypto-shred epoch 1 mid-sequence.
            coord
                .b16_key_rotation
                .rotate_to_epoch(2, hex64_from_seed(404))
                .expect("rotate b16 epoch 2");
            coord
                .b16_key_rotation
                .shred_retired_epoch(1, "adv-l4-crypto-shred")
                .expect("crypto-shred epoch 1");
        }
        claim.b16_crypto_write.write_epoch = if i >= 4 { 2 } else { 1 };
        claim.b17_hybrid_envelope.envelope_id = format!("env-seq-{i}");
        claim.b17_hybrid_envelope.ots_leaf_index = 10 + i;

        let zk_nonce = format!("zk-seq-{i}");
        coord
            .b18_zk_disclosure
            .issue_challenge_nonce(zk_nonce.clone());
        claim.b18_zk_disclosure.proof_id = format!("zk-proof-seq-{i}");
        claim.b18_zk_disclosure.verifier_challenge_nonce = zk_nonce;
        claim.b18_zk_disclosure.nullifier_hex = hex64_from_seed(9000 + i);
        claim.b20_federated_op.op_id = format!("fed-seq-{i}");

        let cert = coord.admit_release(&claim).expect("sequential release");
        assert_eq!(cert.b9_log_leaf_index, i as usize);
        assert_eq!(cert.b9_tree_head.tree_size, (i + 1) as usize);
        tree_heads.push(cert.b9_tree_head);
        digests.push(cert.composite_digest);
    }

    let final_head = tree_heads.last().expect("final head");
    // Verify RFC 6962 inclusion proof for every single leaf against the final tree head.
    for (idx, digest) in digests.iter().enumerate() {
        let proof = coord
            .b9_transparency_log
            .inclusion_proof(idx)
            .expect("inclusion proof");
        verify_inclusion_proof(digest.as_bytes(), &proof, final_head)
            .expect("inclusion proof must verify");
    }
    // Verify RFC 6962 consistency proofs from every historical tree head to the final head.
    for prev in &tree_heads[..tree_heads.len() - 1] {
        let consistency = coord
            .b9_transparency_log
            .consistency_proof(prev.tree_size)
            .expect("consistency proof");
        verify_tree_head_progression(prev, final_head, &consistency)
            .expect("tree head progression must verify");
    }

    // Cross-pillar replay resistance checks after 8 committed releases:
    // 1. Replaying terminal B13 case fails closed.
    let mut replay = base_claim.clone();
    replay.b13_arbitration.case_id = "arb-seq-0".to_owned();
    assert!(matches!(
        coord.admit_release(&replay),
        Err(UnifiedDefenseRejection::B13FederatedArbitration(_))
    ));

    // 2. Writing to crypto-shredded B16 epoch 1 fails closed.
    replay.b13_arbitration.case_id = "arb-fresh-99".to_owned();
    coord
        .b15_confidential_enclave
        .issue_challenge_nonce("nonce-fresh-99");
    replay.b15_enclave_quote.challenge_nonce = "nonce-fresh-99".to_owned();
    replay.b16_crypto_write.write_epoch = 1;
    assert!(matches!(
        coord.admit_release(&replay),
        Err(UnifiedDefenseRejection::B16KeyRotation(_))
    ));

    // 3. Reusing B17 OTS leaf index 10 fails closed.
    replay.b16_crypto_write.write_epoch = 2;
    replay.b17_hybrid_envelope.ots_leaf_index = 10;
    assert!(matches!(
        coord.admit_release(&replay),
        Err(UnifiedDefenseRejection::B17PostQuantum(_))
    ));

    // 4. Replaying burned B18 nullifier fails closed even with a fresh challenge.
    replay.b17_hybrid_envelope.ots_leaf_index = 999;
    coord.b18_zk_disclosure.issue_challenge_nonce("zk-fresh-99");
    replay.b18_zk_disclosure.verifier_challenge_nonce = "zk-fresh-99".to_owned();
    replay.b18_zk_disclosure.nullifier_hex = hex64_from_seed(9000);
    assert!(matches!(
        coord.admit_release(&replay),
        Err(UnifiedDefenseRejection::B18ZkSelectiveDisclosure(_))
    ));

    // Tree size must remain strictly 8 across all 4 replay attempts.
    assert_eq!(coord.b9_transparency_log.tree_head().tree_size, 8);
}

#[test]
fn adv_l4_seeded_hostile_mutation_fuzz_256_rounds_never_panics_or_admits() {
    let (mut coord, base_claim) = build_clean_fixture();
    let mut rng = 0x5eed_cafe_babe_d00du64;

    for round in 0..256u64 {
        rng = splitmix64(rng ^ round);
        let mut hostile = base_claim.clone();
        // Guarantee at least one pillar is sabotaged in every round via deterministic bitmask.
        let mut mask = (rng & 0x3ff) as u16;
        if mask == 0 {
            mask = 1 << (round % 10);
        }

        if mask & (1 << 0) != 0 {
            hostile.b12_distilled_packet.output_secrecy = SecrecyClass::Public;
        }
        if mask & (1 << 1) != 0 {
            hostile.b13_arbitration.wait_for_edges = vec![
                ("n1".to_owned(), "n2".to_owned()),
                ("n2".to_owned(), "n3".to_owned()),
                ("n3".to_owned(), "n1".to_owned()),
            ];
        }
        if mask & (1 << 2) != 0 {
            hostile.b14_supply_chain.builder_b_digest = hex64('0');
        }
        if mask & (1 << 3) != 0 {
            hostile.b15_enclave_quote.tcb_svn = 1;
        }
        if mask & (1 << 4) != 0 {
            hostile.b16_crypto_write.tenant_id = "evil-tenant".to_owned();
        }
        if mask & (1 << 5) != 0 {
            hostile.b17_hybrid_envelope.suite = CryptoSuite::ClassicalOnlyEd25519;
        }
        if mask & (1 << 6) != 0 {
            hostile.b18_zk_disclosure.sigma_proof_verified = false;
        }
        if mask & (1 << 7) != 0 {
            hostile.b19_geo_placement.target_storage_region = "SANCTIONED-XX".to_owned();
        }
        if mask & (1 << 8) != 0 {
            hostile.b20_federated_op.target_org = hostile.b20_federated_op.origin_org.clone();
        }
        if mask & (1 << 9) != 0 {
            hostile.b21_model_inference.prng_seed = None;
        }

        assert!(
            coord.admit_release(&hostile).is_err(),
            "ADV-L4 round {round} (mask={mask:#x}) must fail closed"
        );
        assert_eq!(coord.b9_transparency_log.tree_head().tree_size, 0);
    }
}
