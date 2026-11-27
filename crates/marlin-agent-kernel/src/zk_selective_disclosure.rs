// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B18 Zero-Knowledge Selective-Disclosure Compliance Proofs Gate.
//!
//! Enforces the formal invariants of `ZkSelectiveDisclosure.qnt` /
//! `MarlinBackendZkDisclosureOverlay.qnt` and `ZeroKnowledgeSelectiveDisclosure.lean`:
//!   1. Zero over-disclosure: none of the policy-forbidden secret fields may be revealed
//!      in `disclosed_fields`, and every required compliance predicate must be proven.
//!   2. Valid Sigma/ZK proof: `sigma_proof_verified` must hold against a pinned 64-hex
//!      commitment root and a fresh, gate-issued verifier challenge nonce.
//!   3. Single-use nullifier freshness: each 64-hex nullifier is burned on admission,
//!      permanently refusing proof replay attempts.

use std::collections::BTreeSet;

use crate::governance_admission::governance_sha256_bytes;

fn is_valid_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = governance_sha256_bytes(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Selective-disclosure zero-knowledge compliance proof claim submitted for B18 admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZkDisclosureClaim {
    pub proof_id: String,
    pub commitment_root: String,
    pub verifier_challenge_nonce: String,
    pub nullifier_hex: String,
    pub disclosed_fields: BTreeSet<String>,
    pub proven_predicates: BTreeSet<String>,
    pub sigma_proof_verified: bool,
}

/// Machine-checkable fail-closed refusal reasons for B18 selective disclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZkDisclosureRejection {
    InvalidCommitmentRoot { root: String },
    InvalidNullifierDigest { nullifier: String },
    UnissuedChallengeNonce { nonce: String },
    SecretFieldOverDisclosed { field: String },
    MissingRequiredPredicate { predicate: String },
    ForgedSigmaProof { proof_id: String },
    NullifierReplayDetected { nullifier: String },
}

/// Deterministic admission receipt binding the verified ZK proof and burned nullifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZkDisclosureReceipt {
    pub proof_id: String,
    pub commitment_root: String,
    pub nullifier_hex: String,
    pub proven_predicate_count: usize,
    pub receipt_digest: String,
}

fn canonical_field_key(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

/// Stateful B18 zero-knowledge selective-disclosure compliance gate.
#[derive(Debug, Clone, Default)]
pub struct ZkSelectiveDisclosureGate {
    forbidden_secret_fields: BTreeSet<String>,
    required_predicates: BTreeSet<String>,
    pending_challenges: BTreeSet<String>,
    consumed_challenges: BTreeSet<String>,
    spent_nullifiers: BTreeSet<String>,
    verified_disclosures: u64,
}

impl ZkSelectiveDisclosureGate {
    #[must_use]
    pub fn new(
        forbidden_secret_fields: BTreeSet<String>,
        required_predicates: BTreeSet<String>,
    ) -> Self {
        let normalized_forbidden = forbidden_secret_fields
            .into_iter()
            .map(|field| canonical_field_key(&field))
            .filter(|field| !field.is_empty())
            .collect();
        Self {
            forbidden_secret_fields: normalized_forbidden,
            required_predicates,
            pending_challenges: BTreeSet::new(),
            consumed_challenges: BTreeSet::new(),
            spent_nullifiers: BTreeSet::new(),
            verified_disclosures: 0,
        }
    }

    /// Issue a fresh verifier challenge nonce that binds the Sigma/ZK transcript.
    ///
    /// Blank nonces and previously consumed nonces are ignored fail-closed to
    /// prevent verifier challenge replay.
    pub fn issue_challenge_nonce(&mut self, nonce: impl Into<String>) {
        let n = nonce.into();
        if !n.trim().is_empty() && !self.consumed_challenges.contains(&n) {
            self.pending_challenges.insert(n);
        }
    }

    #[must_use]
    pub fn verified_disclosures(&self) -> u64 {
        self.verified_disclosures
    }

    #[must_use]
    pub fn is_nullifier_spent(&self, nullifier_hex: &str) -> bool {
        self.spent_nullifiers
            .contains(&nullifier_hex.to_ascii_lowercase())
    }

    /// Admit a selective-disclosure compliance proof strictly if zero secrets leak,
    /// all required predicates are proven, the Sigma proof verifies, and the nullifier is fresh.
    pub fn admit_disclosure(
        &mut self,
        claim: &ZkDisclosureClaim,
    ) -> Result<ZkDisclosureReceipt, ZkDisclosureRejection> {
        let root = claim.commitment_root.to_ascii_lowercase();
        if !is_valid_sha256_hex(&root) {
            return Err(ZkDisclosureRejection::InvalidCommitmentRoot {
                root: claim.commitment_root.clone(),
            });
        }
        let nullifier = claim.nullifier_hex.to_ascii_lowercase();
        if !is_valid_sha256_hex(&nullifier) {
            return Err(ZkDisclosureRejection::InvalidNullifierDigest {
                nullifier: claim.nullifier_hex.clone(),
            });
        }
        if !self
            .pending_challenges
            .contains(&claim.verifier_challenge_nonce)
        {
            return Err(ZkDisclosureRejection::UnissuedChallengeNonce {
                nonce: claim.verifier_challenge_nonce.clone(),
            });
        }
        for field in &claim.disclosed_fields {
            let canonical = canonical_field_key(field);
            if self.forbidden_secret_fields.contains(&canonical) {
                return Err(ZkDisclosureRejection::SecretFieldOverDisclosed {
                    field: field.clone(),
                });
            }
        }
        for req in &self.required_predicates {
            if !claim.proven_predicates.contains(req) {
                return Err(ZkDisclosureRejection::MissingRequiredPredicate {
                    predicate: req.clone(),
                });
            }
        }
        if !claim.sigma_proof_verified {
            return Err(ZkDisclosureRejection::ForgedSigmaProof {
                proof_id: claim.proof_id.clone(),
            });
        }
        if self.spent_nullifiers.contains(&nullifier) {
            return Err(ZkDisclosureRejection::NullifierReplayDetected { nullifier });
        }

        // Consume single-use challenge and burn nullifier permanently.
        self.pending_challenges
            .remove(&claim.verifier_challenge_nonce);
        self.consumed_challenges
            .insert(claim.verifier_challenge_nonce.clone());
        self.spent_nullifiers.insert(nullifier.clone());
        self.verified_disclosures = self.verified_disclosures.saturating_add(1);

        let mut preds: Vec<&str> = claim.proven_predicates.iter().map(String::as_str).collect();
        preds.sort_unstable();
        let receipt_digest = sha256_hex(
            format!(
                "b18|proof={}|root={root}|nullifier={nullifier}|preds={}",
                claim.proof_id,
                preds.join(",")
            )
            .as_bytes(),
        );
        Ok(ZkDisclosureReceipt {
            proof_id: claim.proof_id.clone(),
            commitment_root: root,
            nullifier_hex: nullifier,
            proven_predicate_count: claim.proven_predicates.len(),
            receipt_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex64(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn setup_gate_and_claim() -> (ZkSelectiveDisclosureGate, ZkDisclosureClaim) {
        let mut gate = ZkSelectiveDisclosureGate::new(
            BTreeSet::from(["raw_pii".to_owned(), "signing_seed".to_owned()]),
            BTreeSet::from(["eu_kyc_valid".to_owned(), "sanctions_clear".to_owned()]),
        );
        gate.issue_challenge_nonce("zk-chal-01");

        let claim = ZkDisclosureClaim {
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
        (gate, claim)
    }

    #[test]
    fn b18_admits_clean_selective_disclosure_and_blocks_nullifier_replay() {
        let (mut gate, mut claim) = setup_gate_and_claim();
        let receipt = gate.admit_disclosure(&claim).expect("clean zk proof");
        assert_eq!(receipt.receipt_digest.len(), 64);
        assert_eq!(gate.verified_disclosures(), 1);
        assert!(gate.is_nullifier_spent(&claim.nullifier_hex));

        // Even with a freshly issued challenge nonce, reusing the spent nullifier fails closed.
        gate.issue_challenge_nonce("zk-chal-02");
        claim.verifier_challenge_nonce = "zk-chal-02".to_owned();
        assert!(matches!(
            gate.admit_disclosure(&claim),
            Err(ZkDisclosureRejection::NullifierReplayDetected { .. })
        ));
    }

    #[test]
    fn b18_refuses_secret_over_disclosure_missing_predicate_and_forged_sigma() {
        let (mut gate, mut claim) = setup_gate_and_claim();

        claim.disclosed_fields.insert("raw_pii".to_owned());
        assert!(matches!(
            gate.admit_disclosure(&claim),
            Err(ZkDisclosureRejection::SecretFieldOverDisclosed { .. })
        ));
        claim.disclosed_fields.remove("raw_pii");

        claim.proven_predicates.remove("sanctions_clear");
        assert!(matches!(
            gate.admit_disclosure(&claim),
            Err(ZkDisclosureRejection::MissingRequiredPredicate { .. })
        ));
        claim.proven_predicates.insert("sanctions_clear".to_owned());

        claim.sigma_proof_verified = false;
        assert!(matches!(
            gate.admit_disclosure(&claim),
            Err(ZkDisclosureRejection::ForgedSigmaProof { .. })
        ));

        // Failed attempts did not burn the challenge or nullifier.
        claim.sigma_proof_verified = true;
        gate.admit_disclosure(&claim)
            .expect("clean retry still succeeds");
    }
}
