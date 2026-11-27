// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unified Fail-Closed Defense Pillar Coordinator (`B9` + `B12` + `B14` + `B15` + `B16` + `B17` + `B19` + `B20` + `B21`).
//!
//! Composes all 9 formally verified Rust defense gates into one atomic release admission
//! pipeline whose final composite certificate is appended into the `B9` RFC 6962
//! transparency log only when EVERY enabled pillar passes simultaneously.

use crate::confidential_enclave::{
    ConfidentialEnclaveGate, ConfidentialEnclaveRejection, EnclaveAttestationQuote,
    EnclaveKeyReleaseReceipt,
};
use crate::context_distillation::{
    ContextDistillationGate, ContextDistillationReceipt, ContextDistillationRejection,
    ContextSegment, DistilledContextPacket,
};
use crate::federated_org_reciprocity::{
    FederatedOpClaim, FederatedOpReceipt, FederatedOrgReciprocityGate, FederatedOrgRejection,
};
use crate::geo_residency_sovereignty::{
    GeoPlacementClaim, GeoPlacementReceipt, GeoResidencyRejection, GeoResidencySovereigntyGate,
};
use crate::governance_admission::governance_sha256_bytes;
use crate::key_rotation_shredding::{
    CryptoOpReceipt, CryptoWriteClaim, KeyRotationRejection, KeyRotationShreddingGate,
};
use crate::model_weight_attestation::{
    ModelInferenceAdmissionReceipt, ModelInferenceClaim, ModelWeightAttestationGate,
    ModelWeightRejection,
};
use crate::post_quantum_agility::{
    HybridEnvelopeAdmissionReceipt, HybridSignatureEnvelope, PostQuantumAgilityGate,
    PostQuantumRejection,
};
use crate::supply_chain_provenance::{
    SupplyChainAdmissionReceipt, SupplyChainBuildClaim, SupplyChainProvenanceGate,
    SupplyChainRejection,
};
use crate::transparency_log::{GovernanceTransparencyLog, GovernanceTreeHead};

fn sha256_hex(data: &[u8]) -> String {
    let digest = governance_sha256_bytes(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Complete multi-pillar release evaluation bundle across B12/B14/B15/B16/B17/B19/B20/B21.
#[derive(Debug, Clone)]
pub struct UnifiedDefenseReleaseClaim {
    pub release_id: String,
    pub b12_source_segments: Vec<ContextSegment>,
    pub b12_distilled_packet: DistilledContextPacket,
    pub b14_supply_chain: SupplyChainBuildClaim,
    pub b15_enclave_quote: EnclaveAttestationQuote,
    pub b16_crypto_write: CryptoWriteClaim,
    pub b17_hybrid_envelope: HybridSignatureEnvelope,
    pub b19_geo_placement: GeoPlacementClaim,
    pub b20_federated_op: FederatedOpClaim,
    pub b21_model_inference: ModelInferenceClaim,
}

/// Typed fail-closed refusal pinpointing the exact defense pillar that blocked release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedDefenseRejection {
    B12ContextDistillation(ContextDistillationRejection),
    B14SupplyChain(SupplyChainRejection),
    B15ConfidentialEnclave(ConfidentialEnclaveRejection),
    B16KeyRotation(KeyRotationRejection),
    B17PostQuantum(PostQuantumRejection),
    B19GeoResidency(GeoResidencyRejection),
    B20FederatedOrg(FederatedOrgRejection),
    B21ModelWeight(ModelWeightRejection),
}

/// Composite 9-pillar release certificate anchored into the B9 RFC 6962 transparency log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedDefenseReleaseCertificate {
    pub release_id: String,
    pub b12_receipt: ContextDistillationReceipt,
    pub b14_receipt: SupplyChainAdmissionReceipt,
    pub b15_receipt: EnclaveKeyReleaseReceipt,
    pub b16_receipt: CryptoOpReceipt,
    pub b17_receipt: HybridEnvelopeAdmissionReceipt,
    pub b19_receipt: GeoPlacementReceipt,
    pub b20_receipt: FederatedOpReceipt,
    pub b21_receipt: ModelInferenceAdmissionReceipt,
    pub composite_digest: String,
    pub b9_log_leaf_index: usize,
    pub b9_tree_head: GovernanceTreeHead,
}

/// Single fail-closed coordinator owning all 9 formal defense gates (`B9`..`B21`).
#[derive(Debug, Clone)]
pub struct UnifiedDefensePillarCoordinator {
    pub b9_transparency_log: GovernanceTransparencyLog,
    pub b12_context_distillation: ContextDistillationGate,
    pub b14_supply_chain: SupplyChainProvenanceGate,
    pub b15_confidential_enclave: ConfidentialEnclaveGate,
    pub b16_key_rotation: KeyRotationShreddingGate,
    pub b17_post_quantum: PostQuantumAgilityGate,
    pub b19_geo_residency: GeoResidencySovereigntyGate,
    pub b20_federated_org: FederatedOrgReciprocityGate,
    pub b21_model_weight: ModelWeightAttestationGate,
}

impl UnifiedDefensePillarCoordinator {
    /// Atomically evaluate all 8 admission pillars and commit the composite certificate
    /// to the B9 RFC 6962 transparency log ONLY if every single pillar succeeds.
    ///
    /// Evaluates against cloned gate state first so any failure leaves all stateful gates
    /// and the B9 transparency log untouched (strict transactional rollback semantics).
    pub fn admit_release(
        &mut self,
        claim: &UnifiedDefenseReleaseClaim,
    ) -> Result<UnifiedDefenseReleaseCertificate, UnifiedDefenseRejection> {
        let b12_receipt = self
            .b12_context_distillation
            .admit_distillation(&claim.b12_source_segments, &claim.b12_distilled_packet)
            .map_err(UnifiedDefenseRejection::B12ContextDistillation)?;

        let mut next_b14 = self.b14_supply_chain.clone();
        let b14_receipt = next_b14
            .admit_build(&claim.b14_supply_chain)
            .map_err(UnifiedDefenseRejection::B14SupplyChain)?;

        let mut next_b15 = self.b15_confidential_enclave.clone();
        let b15_receipt = next_b15
            .release_sealed_key(&claim.b15_enclave_quote)
            .map_err(UnifiedDefenseRejection::B15ConfidentialEnclave)?;

        let mut next_b16 = self.b16_key_rotation.clone();
        let b16_receipt = next_b16
            .admit_write(&claim.b16_crypto_write)
            .map_err(UnifiedDefenseRejection::B16KeyRotation)?;

        let mut next_b17 = self.b17_post_quantum.clone();
        let b17_receipt = next_b17
            .admit_envelope(&claim.b17_hybrid_envelope)
            .map_err(UnifiedDefenseRejection::B17PostQuantum)?;

        let mut next_b19 = self.b19_geo_residency.clone();
        let b19_receipt = next_b19
            .admit_placement(&claim.b19_geo_placement)
            .map_err(UnifiedDefenseRejection::B19GeoResidency)?;

        let mut next_b20 = self.b20_federated_org.clone();
        let b20_receipt = next_b20
            .admit_federated_op(&claim.b20_federated_op)
            .map_err(UnifiedDefenseRejection::B20FederatedOrg)?;

        let b21_receipt = self
            .b21_model_weight
            .admit_inference(&claim.b21_model_inference)
            .map_err(UnifiedDefenseRejection::B21ModelWeight)?;

        let composite_payload = format!(
            "marlin-unified-v1|rel={}|b12={}|b14={}|b15={}|b16={}|b17={}|b19={}|b20={}|b21={}",
            claim.release_id,
            b12_receipt.provenance_digest.as_str(),
            b14_receipt.receipt_digest,
            b15_receipt.receipt_digest,
            b16_receipt.receipt_digest,
            b17_receipt.receipt_digest,
            b19_receipt.receipt_digest,
            b20_receipt.receipt_digest,
            b21_receipt.attestation_digest.as_str(),
        );
        let composite_digest = sha256_hex(composite_payload.as_bytes());

        // All 8 admission pillars succeeded: atomically commit gate states & append B9 leaf.
        self.b14_supply_chain = next_b14;
        self.b15_confidential_enclave = next_b15;
        self.b16_key_rotation = next_b16;
        self.b17_post_quantum = next_b17;
        self.b19_geo_residency = next_b19;
        self.b20_federated_org = next_b20;

        let b9_log_leaf_index = self
            .b9_transparency_log
            .append(composite_digest.as_bytes().to_vec());
        let b9_tree_head = self.b9_transparency_log.tree_head();

        Ok(UnifiedDefenseReleaseCertificate {
            release_id: claim.release_id.clone(),
            b12_receipt,
            b14_receipt,
            b15_receipt,
            b16_receipt,
            b17_receipt,
            b19_receipt,
            b20_receipt,
            b21_receipt,
            composite_digest,
            b9_log_leaf_index,
            b9_tree_head,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::context_distillation::{EpistemicIntegrity, SecrecyClass};
    use crate::federated_org_reciprocity::BilateralTreaty;
    use crate::governance_admission::{GovernanceProofDigest, GovernedSubjectName};
    use crate::model_weight_attestation::{ModelWeightGovernancePolicy, compute_shard_merkle_root};
    use crate::post_quantum_agility::CryptoSuite;
    use crate::supply_chain_provenance::SupplyChainDependencySpec;
    use crate::transparency_log::verify_inclusion_proof;

    fn hex64(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn proof_digest(ch: char) -> GovernanceProofDigest {
        GovernanceProofDigest::new(format!("sha256:{}", hex64(ch))).expect("valid proof digest")
    }

    fn subject(name: &str) -> GovernedSubjectName {
        GovernedSubjectName::new(name).expect("valid governed subject")
    }

    fn build_clean_coordinator_and_claim() -> (
        UnifiedDefensePillarCoordinator,
        UnifiedDefenseReleaseClaim,
    ) {
        let mut b14 = SupplyChainProvenanceGate::new();
        b14.register_pinned_dependency("marlin-agent-kernel", "0.4.0", hex64('a'))
            .expect("pin b14");

        let mut b15 = ConfidentialEnclaveGate::new(14);
        b15.register_measurement(hex64('e'), Some(hex64('f')))
            .expect("pin b15");
        b15.issue_challenge_nonce("nonce-unified-01");

        let b16 = KeyRotationShreddingGate::new("tenant-eu", hex64('1')).expect("init b16");
        let b17 = PostQuantumAgilityGate::default();

        let mut b19 = GeoResidencySovereigntyGate::new();
        b19.allow_storage_region("EU-DE");
        b19.allow_inference_region("EU-DE");
        b19.trust_authority("eu-dpa-1");

        let mut b20 = FederatedOrgReciprocityGate::new();
        b20.register_treaty(BilateralTreaty {
            treaty_id: "treaty-eu-01".to_owned(),
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
            b14_supply_chain: b14,
            b15_confidential_enclave: b15,
            b16_key_rotation: b16,
            b17_post_quantum: b17,
            b19_geo_residency: b19,
            b20_federated_org: b20,
            b21_model_weight: b21,
        };

        let seg_id = subject("seg-policy");
        let seg_digest = proof_digest('7');
        let claim = UnifiedDefenseReleaseClaim {
            release_id: "rel-unified-001".to_owned(),
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
            b14_supply_chain: SupplyChainBuildClaim {
                release_id: "rel-unified-001".to_owned(),
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
                challenge_nonce: "nonce-unified-01".to_owned(),
                bound_b14_artifact_digest: Some(hex64('f')),
                quote_signature_verified: true,
            },
            b16_crypto_write: CryptoWriteClaim {
                tenant_id: "tenant-eu".to_owned(),
                write_epoch: 1,
                plaintext_digest: hex64('c'),
            },
            b17_hybrid_envelope: HybridSignatureEnvelope {
                envelope_id: "env-unified-01".to_owned(),
                suite: CryptoSuite::HybridEd25519MlDsa65,
                domain_tag: "marlin.unified.release.v1".to_owned(),
                payload_digest: hex64('f'),
                classical_leg_verified: true,
                pq_leg_verified: true,
                ots_leaf_index: 1,
            },
            b19_geo_placement: GeoPlacementClaim {
                placement_id: "geo-001".to_owned(),
                tenant_id: "tenant-eu".to_owned(),
                source_region: "EU-DE".to_owned(),
                target_storage_region: "EU-DE".to_owned(),
                inference_region: "EU-DE".to_owned(),
                observed_unix_ms: 1_700_000_000_000,
                cross_border_permit: None,
            },
            b20_federated_op: FederatedOpClaim {
                op_id: "fed-001".to_owned(),
                treaty_id: "treaty-eu-01".to_owned(),
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
    fn unified_nine_pillar_release_admits_and_anchors_verifiable_b9_inclusion_proof() {
        let (mut coord, claim) = build_clean_coordinator_and_claim();
        let cert = coord.admit_release(&claim).expect("all 9 pillars pass");
        assert_eq!(cert.b9_log_leaf_index, 0);
        assert_eq!(cert.b9_tree_head.tree_size, 1);

        let proof = coord
            .b9_transparency_log
            .inclusion_proof(cert.b9_log_leaf_index)
            .expect("inclusion proof");
        verify_inclusion_proof(cert.composite_digest.as_bytes(), &proof, &cert.b9_tree_head)
            .expect("B9 inclusion proof must verify");
    }

    #[test]
    fn unified_coordinator_fails_closed_atomically_if_any_single_pillar_fails() {
        let (mut coord, mut claim) = build_clean_coordinator_and_claim();
        // Sabotage ONLY B17 (single-leg PQ bypass); B14/B15/B16 state and B9 log must remain untouched!
        claim.b17_hybrid_envelope.pq_leg_verified = false;
        assert!(matches!(
            coord.admit_release(&claim),
            Err(UnifiedDefenseRejection::B17PostQuantum(_))
        ));
        assert_eq!(coord.b9_transparency_log.tree_head().tree_size, 0);
        assert_eq!(coord.b15_confidential_enclave.keys_released(), 0);

        // Fixing B17 allows the exact same B15 challenge nonce to succeed (proving atomic rollback!).
        claim.b17_hybrid_envelope.pq_leg_verified = true;
        let cert = coord
            .admit_release(&claim)
            .expect("transactional rollback preserved fresh B15 nonce");
        assert_eq!(cert.b9_tree_head.tree_size, 1);
    }
}
