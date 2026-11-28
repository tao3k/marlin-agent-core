// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Rust B21 Deterministic Model-Weight Pinning & Reproducible Inference
//! Attestation Gate.
//!
//! Refined by:
//! - `engineering.defense.model-weight-inference-attestation` (`ModelWeightInferenceAttestation.qnt`)
//! - `DeterministicEngineering.Engineering.ModelWeightAndInferenceAttestation` (Lean 4 / LeanPoo)
//! - `dev/quint/threat/MarlinBackendModelWeightOverlay.qnt`
//!
//! Enforces fail-closed cryptographic pinning of:
//! 1. Base model tensor-shard Merkle root (`sha256:<64 lowercase hex>`),
//! 2. Explicit LoRA / fine-tune adapter digest allowlist (zero stealth adapters),
//! 3. Serving kernel / runtime digest parity (prevents silent quantization or runtime drift),
//! 4. Deterministic greedy sampling (`temperature_milli == 0`) & explicit PRNG seed when locked,
//! 5. Active model-lineage revocation tombstones.

use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
    fmt::Write as _,
};

use crate::governance_admission::{
    GovernanceAdmissionError, GovernanceProofDigest, GovernedSubjectName, governance_sha256_bytes,
};

fn sha256_prefixed_digest(bytes: &[u8]) -> GovernanceProofDigest {
    let raw = governance_sha256_bytes(bytes);
    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in raw {
        let _ = write!(&mut hex, "{b:02x}");
    }
    GovernanceProofDigest::new(hex).expect("32-byte SHA-256 hex always forms a valid digest")
}

fn largest_power_of_two_less_than(n: usize) -> usize {
    debug_assert!(n >= 2);
    1usize << (usize::BITS - 1 - (n - 1).leading_zeros())
}

fn shard_mth(leaves: &[[u8; 32]]) -> [u8; 32] {
    match leaves.len() {
        1 => leaves[0],
        n => {
            let k = largest_power_of_two_less_than(n);
            let left = shard_mth(&leaves[..k]);
            let right = shard_mth(&leaves[k..]);
            let mut buf = [0u8; 16 + 64];
            buf[..16].copy_from_slice(b"marlin:b21:node:");
            buf[16..48].copy_from_slice(&left);
            buf[48..80].copy_from_slice(&right);
            governance_sha256_bytes(&buf)
        }
    }
}

/// Computes a deterministic RFC 6962-structured binary Merkle root (`sha256:<64 hex>`) over
/// non-empty tensor shard digests in sorted order (domain-separated leaf & internal nodes,
/// never duplicating trailing odd nodes).
pub fn compute_shard_merkle_root(
    shard_digests: &[GovernanceProofDigest],
) -> Result<GovernanceProofDigest, ModelWeightRejection> {
    if shard_digests.is_empty() {
        return Err(ModelWeightRejection::EmptyWeightShards);
    }
    let mut sorted: Vec<&str> = shard_digests.iter().map(GovernanceProofDigest::as_str).collect();
    sorted.sort_unstable();

    let leaves: Vec<[u8; 32]> = sorted
        .into_iter()
        .map(|digest| {
            let mut buf = Vec::with_capacity(16 + digest.len());
            buf.extend_from_slice(b"marlin:b21:leaf:");
            buf.extend_from_slice(digest.as_bytes());
            governance_sha256_bytes(&buf)
        })
        .collect();

    let root = shard_mth(&leaves);

    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in root {
        let _ = write!(&mut hex, "{b:02x}");
    }
    GovernanceProofDigest::new(hex).map_err(ModelWeightRejection::InvalidDigest)
}

/// Governance policy pinning base model weights, allowlisted adapters, serving kernel,
/// and deterministic sampling requirements (B21).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWeightGovernancePolicy {
    pub model_id: GovernedSubjectName,
    pub pinned_weight_root: GovernanceProofDigest,
    pub allowlisted_adapters: BTreeSet<GovernanceProofDigest>,
    pub pinned_serving_kernel_digest: GovernanceProofDigest,
    pub require_deterministic_sampling: bool,
    pub max_temperature_milli: u32,
    pub revoked_models: BTreeSet<GovernedSubjectName>,
}

/// Inference attestation claim presented at runtime before admitting model execution or output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelInferenceClaim {
    pub model_id: GovernedSubjectName,
    pub observed_weight_root: GovernanceProofDigest,
    pub active_adapters: Vec<GovernanceProofDigest>,
    pub observed_serving_kernel_digest: GovernanceProofDigest,
    pub temperature_milli: u32,
    pub prng_seed: Option<u64>,
}

/// Typed, fail-closed rejection reasons for B21 model-weight & inference attestation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelWeightRejection {
    EmptyWeightShards,
    InvalidDigest(GovernanceAdmissionError),
    ModelLineageRevoked(GovernedSubjectName),
    ModelIdentityMismatch {
        expected: GovernedSubjectName,
        observed: GovernedSubjectName,
    },
    UnpinnedBaseWeightRoot {
        expected: GovernanceProofDigest,
        observed: GovernanceProofDigest,
    },
    StealthOrUnallowlistedAdapter(GovernanceProofDigest),
    ServingKernelDrift {
        expected: GovernanceProofDigest,
        observed: GovernanceProofDigest,
    },
    NonDeterministicTemperature {
        observed_milli: u32,
        max_allowed_milli: u32,
    },
    MissingDeterministicSeed,
}

impl fmt::Display for ModelWeightRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyWeightShards => write!(f, "tensor shard list must not be empty"),
            Self::InvalidDigest(err) => write!(f, "invalid weight digest: {err}"),
            Self::ModelLineageRevoked(id) => write!(f, "model lineage '{id}' is revoked"),
            Self::ModelIdentityMismatch { expected, observed } => {
                write!(f, "model identity mismatch: expected '{expected}', observed '{observed}'")
            }
            Self::UnpinnedBaseWeightRoot { expected, observed } => {
                write!(
                    f,
                    "unpinned base weight Merkle root: expected {}, observed {}",
                    expected.as_str(),
                    observed.as_str()
                )
            }
            Self::StealthOrUnallowlistedAdapter(digest) => {
                write!(f, "stealth or unallowlisted LoRA adapter: {}", digest.as_str())
            }
            Self::ServingKernelDrift { expected, observed } => {
                write!(
                    f,
                    "serving kernel digest drift: expected {}, observed {}",
                    expected.as_str(),
                    observed.as_str()
                )
            }
            Self::NonDeterministicTemperature {
                observed_milli,
                max_allowed_milli,
            } => write!(
                f,
                "non-deterministic sampling temperature {observed_milli}m exceeds limit {max_allowed_milli}m"
            ),
            Self::MissingDeterministicSeed => {
                write!(f, "deterministic inference mode requires an explicit PRNG seed")
            }
        }
    }
}

impl Error for ModelWeightRejection {}

/// Cryptographically bound admission receipt emitted only after all B21 checks pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelInferenceAdmissionReceipt {
    pub model_id: GovernedSubjectName,
    pub weight_root: GovernanceProofDigest,
    pub adapter_count: usize,
    pub serving_kernel_digest: GovernanceProofDigest,
    pub temperature_milli: u32,
    pub prng_seed: Option<u64>,
    pub attestation_digest: GovernanceProofDigest,
}

/// Authoritative B21 fail-closed model-weight and inference attestation gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWeightAttestationGate {
    policy: ModelWeightGovernancePolicy,
}

impl ModelWeightAttestationGate {
    pub fn new(policy: ModelWeightGovernancePolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &ModelWeightGovernancePolicy {
        &self.policy
    }

    /// Permanently revokes a model lineage so subsequent inference claims fail closed.
    pub fn revoke_model(&mut self, model_id: GovernedSubjectName) {
        self.policy.revoked_models.insert(model_id);
    }

    /// Evaluates `claim` against the pinned `ModelWeightGovernancePolicy` fail-closed.
    pub fn admit_inference(
        &self,
        claim: &ModelInferenceClaim,
    ) -> Result<ModelInferenceAdmissionReceipt, ModelWeightRejection> {
        if self.policy.revoked_models.contains(&claim.model_id) {
            return Err(ModelWeightRejection::ModelLineageRevoked(
                claim.model_id.clone(),
            ));
        }
        if claim.model_id != self.policy.model_id {
            return Err(ModelWeightRejection::ModelIdentityMismatch {
                expected: self.policy.model_id.clone(),
                observed: claim.model_id.clone(),
            });
        }
        if claim.observed_weight_root != self.policy.pinned_weight_root {
            return Err(ModelWeightRejection::UnpinnedBaseWeightRoot {
                expected: self.policy.pinned_weight_root.clone(),
                observed: claim.observed_weight_root.clone(),
            });
        }
        for adapter in &claim.active_adapters {
            if !self.policy.allowlisted_adapters.contains(adapter) {
                return Err(ModelWeightRejection::StealthOrUnallowlistedAdapter(
                    adapter.clone(),
                ));
            }
        }
        if claim.observed_serving_kernel_digest != self.policy.pinned_serving_kernel_digest {
            return Err(ModelWeightRejection::ServingKernelDrift {
                expected: self.policy.pinned_serving_kernel_digest.clone(),
                observed: claim.observed_serving_kernel_digest.clone(),
            });
        }
        if self.policy.require_deterministic_sampling {
            if claim.temperature_milli > self.policy.max_temperature_milli {
                return Err(ModelWeightRejection::NonDeterministicTemperature {
                    observed_milli: claim.temperature_milli,
                    max_allowed_milli: self.policy.max_temperature_milli,
                });
            }
            if claim.prng_seed.is_none() {
                return Err(ModelWeightRejection::MissingDeterministicSeed);
            }
        }

        let mut canonical = Vec::new();
        canonical.extend_from_slice(b"marlin:b21:receipt:v1\0");
        canonical.extend_from_slice(claim.model_id.as_str().as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(claim.observed_weight_root.as_str().as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(claim.observed_serving_kernel_digest.as_str().as_bytes());
        canonical.push(0);
        let mut sorted_adapters: Vec<&str> = claim
            .active_adapters
            .iter()
            .map(GovernanceProofDigest::as_str)
            .collect();
        sorted_adapters.sort_unstable();
        for ad in sorted_adapters {
            canonical.extend_from_slice(ad.as_bytes());
            canonical.push(b',');
        }
        canonical.extend_from_slice(&claim.temperature_milli.to_be_bytes());
        if let Some(seed) = claim.prng_seed {
            canonical.push(1);
            canonical.extend_from_slice(&seed.to_be_bytes());
        } else {
            canonical.push(0);
        }

        Ok(ModelInferenceAdmissionReceipt {
            model_id: claim.model_id.clone(),
            weight_root: claim.observed_weight_root.clone(),
            adapter_count: claim.active_adapters.len(),
            serving_kernel_digest: claim.observed_serving_kernel_digest.clone(),
            temperature_milli: claim.temperature_milli,
            prng_seed: claim.prng_seed,
            attestation_digest: sha256_prefixed_digest(&canonical),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_of(label: &str) -> GovernanceProofDigest {
        sha256_prefixed_digest(label.as_bytes())
    }

    fn base_policy() -> (ModelWeightAttestationGate, Vec<GovernanceProofDigest>, GovernanceProofDigest, GovernanceProofDigest) {
        let shards = vec![
            digest_of("tensor-shard-0001"),
            digest_of("tensor-shard-0002"),
            digest_of("tensor-shard-0003"),
        ];
        let root = compute_shard_merkle_root(&shards).expect("merkle root");
        let allowlisted_lora = digest_of("lora-safety-v1");
        let kernel = digest_of("vllm-fp16-deterministic-kernel-v1");
        let mut adapters = BTreeSet::new();
        adapters.insert(allowlisted_lora.clone());

        let policy = ModelWeightGovernancePolicy {
            model_id: GovernedSubjectName::new("marlin-gov-70b-v1").unwrap(),
            pinned_weight_root: root,
            allowlisted_adapters: adapters,
            pinned_serving_kernel_digest: kernel.clone(),
            require_deterministic_sampling: true,
            max_temperature_milli: 0,
            revoked_models: BTreeSet::new(),
        };
        (ModelWeightAttestationGate::new(policy), shards, allowlisted_lora, kernel)
    }

    #[test]
    fn admits_clean_zero_adapter_and_allowlisted_lora_deterministic_inference() {
        let (gate, shards, allowlisted_lora, kernel) = base_policy();
        let root = compute_shard_merkle_root(&shards).unwrap();

        let base_claim = ModelInferenceClaim {
            model_id: GovernedSubjectName::new("marlin-gov-70b-v1").unwrap(),
            observed_weight_root: root.clone(),
            active_adapters: Vec::new(),
            observed_serving_kernel_digest: kernel.clone(),
            temperature_milli: 0,
            prng_seed: Some(42),
        };
        let r1 = gate.admit_inference(&base_claim).expect("clean base inference admitted");
        assert_eq!(r1.adapter_count, 0);
        assert_eq!(r1.weight_root, root);

        let lora_claim = ModelInferenceClaim {
            active_adapters: vec![allowlisted_lora],
            ..base_claim
        };
        let r2 = gate
            .admit_inference(&lora_claim)
            .expect("allowlisted LoRA inference admitted");
        assert_eq!(r2.adapter_count, 1);
        assert_ne!(r1.attestation_digest, r2.attestation_digest);
    }

    #[test]
    fn refuses_all_six_b21_inference_integrity_attacks_fail_closed() {
        let (mut gate, shards, _allowlisted_lora, kernel) = base_policy();
        let root = compute_shard_merkle_root(&shards).unwrap();
        let clean = ModelInferenceClaim {
            model_id: GovernedSubjectName::new("marlin-gov-70b-v1").unwrap(),
            observed_weight_root: root,
            active_adapters: Vec::new(),
            observed_serving_kernel_digest: kernel,
            temperature_milli: 0,
            prng_seed: Some(42),
        };

        // 1. Single tensor shard swap (unpinned weight root)
        let mut tampered_shards = shards;
        tampered_shards[1] = digest_of("tensor-shard-0002-poisoned");
        let tampered_root = compute_shard_merkle_root(&tampered_shards).unwrap();
        let mut swapped_claim = clean.clone();
        swapped_claim.observed_weight_root = tampered_root;
        assert!(matches!(
            gate.admit_inference(&swapped_claim),
            Err(ModelWeightRejection::UnpinnedBaseWeightRoot { .. })
        ));

        // 2. Stealth / unallowlisted LoRA adapter
        let mut stealth_claim = clean.clone();
        stealth_claim.active_adapters = vec![digest_of("lora-stealth-backdoor")];
        assert!(matches!(
            gate.admit_inference(&stealth_claim),
            Err(ModelWeightRejection::StealthOrUnallowlistedAdapter(_))
        ));

        // 3. Serving kernel / quantization drift
        let mut drifted_kernel = clean.clone();
        drifted_kernel.observed_serving_kernel_digest = digest_of("int4-unverified-kernel");
        assert!(matches!(
            gate.admit_inference(&drifted_kernel),
            Err(ModelWeightRejection::ServingKernelDrift { .. })
        ));

        // 4. Non-zero sampling temperature
        let mut hot_temp = clean.clone();
        hot_temp.temperature_milli = 700;
        assert!(matches!(
            gate.admit_inference(&hot_temp),
            Err(ModelWeightRejection::NonDeterministicTemperature { .. })
        ));

        // 5. Missing explicit PRNG seed
        let mut no_seed = clean.clone();
        no_seed.prng_seed = None;
        assert_eq!(
            gate.admit_inference(&no_seed),
            Err(ModelWeightRejection::MissingDeterministicSeed)
        );

        // 6. Revoked model lineage
        gate.revoke_model(GovernedSubjectName::new("marlin-gov-70b-v1").unwrap());
        assert!(matches!(
            gate.admit_inference(&clean),
            Err(ModelWeightRejection::ModelLineageRevoked(_))
        ));
    }
}
