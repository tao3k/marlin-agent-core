// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B17 Post-Quantum Hybrid Cryptographic Agility & Anti-Downgrade Gate.
//!
//! Enforces the formal invariants of `PostQuantumCryptoAgility.qnt` /
//! `MarlinBackendPostQuantumOverlay.qnt` and `PostQuantumCryptoAgility.lean`:
//!   1. Active suite must satisfy the monotonic hybrid security floor (`level >= min_suite_level`)
//!      and must not belong to the deprecated suite registry (anti-downgrade).
//!   2. Both classical (e.g. Ed25519) AND post-quantum (e.g. ML-DSA-65 / SLH-DSA) signature legs
//!      must verify simultaneously (strict AND composition; blocks single-leg bypass).
//!   3. Every stateful hash-based OTS leaf index may be consumed at most once (blocks OTS leaf reuse).
//!   4. Domain separation tags and 64-hex SHA-256 payload digests are strictly validated.

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

/// Cryptographic signature suite ordered by post-quantum security floor level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CryptoSuite {
    /// Classical-only Ed25519 (level 1 — rejected under hybrid PQ floor >= 2).
    ClassicalOnlyEd25519,
    /// Hybrid Ed25519 + ML-DSA-65 (FIPS 204, level 2).
    HybridEd25519MlDsa65,
    /// Hybrid Ed25519 + SLH-DSA-SHA2-128s (FIPS 205, level 3).
    HybridEd25519SlhDsa128s,
}

impl CryptoSuite {
    #[must_use]
    pub const fn security_level(self) -> u8 {
        match self {
            Self::ClassicalOnlyEd25519 => 1,
            Self::HybridEd25519MlDsa65 => 2,
            Self::HybridEd25519SlhDsa128s => 3,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClassicalOnlyEd25519 => "classical-ed25519",
            Self::HybridEd25519MlDsa65 => "hybrid-ed25519-mldsa65",
            Self::HybridEd25519SlhDsa128s => "hybrid-ed25519-slhdsa128s",
        }
    }
}

/// Verified dual-leg hybrid signature envelope submitted for B17 admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSignatureEnvelope {
    pub envelope_id: String,
    pub suite: CryptoSuite,
    pub domain_tag: String,
    pub payload_digest: String,
    pub classical_leg_verified: bool,
    pub pq_leg_verified: bool,
    pub ots_leaf_index: u64,
}

/// Machine-checkable refusal reasons for B17 post-quantum agility enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostQuantumRejection {
    SuiteDowngradeRefused {
        suite: CryptoSuite,
        suite_level: u8,
        min_required_level: u8,
    },
    DeprecatedSuiteRefused {
        suite: CryptoSuite,
    },
    SingleLegBypassRefused {
        classical_leg_verified: bool,
        pq_leg_verified: bool,
    },
    OtsLeafReuseDetected {
        ots_leaf_index: u64,
    },
    InvalidDomainTag,
    InvalidPayloadDigest {
        digest: String,
    },
}

/// Deterministic admission receipt binding suite, domain, payload, and consumed OTS leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridEnvelopeAdmissionReceipt {
    pub envelope_id: String,
    pub suite: CryptoSuite,
    pub ots_leaf_index: u64,
    pub receipt_digest: String,
}

/// Stateful B17 post-quantum agility gate with monotonic floor & OTS leaf tracking.
#[derive(Debug, Clone)]
pub struct PostQuantumAgilityGate {
    min_suite_level: u8,
    deprecated_suites: BTreeSet<CryptoSuite>,
    consumed_ots_leaves: BTreeSet<u64>,
    verified_count: u64,
}

impl Default for PostQuantumAgilityGate {
    fn default() -> Self {
        Self::new(2)
    }
}

impl PostQuantumAgilityGate {
    #[must_use]
    pub fn new(min_suite_level: u8) -> Self {
        Self {
            min_suite_level: min_suite_level.max(2),
            deprecated_suites: BTreeSet::new(),
            consumed_ots_leaves: BTreeSet::new(),
            verified_count: 0,
        }
    }

    /// Monotonically raise the minimum security level (never lowers the floor).
    pub fn raise_min_suite_level(&mut self, new_level: u8) {
        if new_level > self.min_suite_level {
            self.min_suite_level = new_level;
        }
    }

    /// Permanently deprecate a cryptographic suite.
    pub fn deprecate_suite(&mut self, suite: CryptoSuite) {
        self.deprecated_suites.insert(suite);
    }

    #[must_use]
    pub fn min_suite_level(&self) -> u8 {
        self.min_suite_level
    }

    #[must_use]
    pub fn verified_count(&self) -> u64 {
        self.verified_count
    }

    /// Verify and admit a hybrid envelope fail-closed, consuming its OTS leaf atomically.
    pub fn admit_envelope(
        &mut self,
        env: &HybridSignatureEnvelope,
    ) -> Result<HybridEnvelopeAdmissionReceipt, PostQuantumRejection> {
        if env.domain_tag.trim().is_empty() {
            return Err(PostQuantumRejection::InvalidDomainTag);
        }
        let payload_digest = env.payload_digest.to_ascii_lowercase();
        if !is_valid_sha256_hex(&payload_digest) {
            return Err(PostQuantumRejection::InvalidPayloadDigest {
                digest: env.payload_digest.clone(),
            });
        }
        if self.deprecated_suites.contains(&env.suite) {
            return Err(PostQuantumRejection::DeprecatedSuiteRefused { suite: env.suite });
        }
        if env.suite.security_level() < self.min_suite_level {
            return Err(PostQuantumRejection::SuiteDowngradeRefused {
                suite: env.suite,
                suite_level: env.suite.security_level(),
                min_required_level: self.min_suite_level,
            });
        }
        if !env.classical_leg_verified || !env.pq_leg_verified {
            return Err(PostQuantumRejection::SingleLegBypassRefused {
                classical_leg_verified: env.classical_leg_verified,
                pq_leg_verified: env.pq_leg_verified,
            });
        }
        if self.consumed_ots_leaves.contains(&env.ots_leaf_index) {
            return Err(PostQuantumRejection::OtsLeafReuseDetected {
                ots_leaf_index: env.ots_leaf_index,
            });
        }

        self.consumed_ots_leaves.insert(env.ots_leaf_index);
        self.verified_count = self.verified_count.saturating_add(1);
        let receipt_digest = sha256_hex(
            format!(
                "b17|id={}|suite={}|domain={}|payload={payload_digest}|ots={}",
                env.envelope_id,
                env.suite.as_str(),
                env.domain_tag,
                env.ots_leaf_index
            )
            .as_bytes(),
        );
        Ok(HybridEnvelopeAdmissionReceipt {
            envelope_id: env.envelope_id.clone(),
            suite: env.suite,
            ots_leaf_index: env.ots_leaf_index,
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

    fn clean_env(ots_leaf_index: u64) -> HybridSignatureEnvelope {
        HybridSignatureEnvelope {
            envelope_id: format!("env-{ots_leaf_index}"),
            suite: CryptoSuite::HybridEd25519MlDsa65,
            domain_tag: "marlin.governance.release.v1".to_owned(),
            payload_digest: hex64('a'),
            classical_leg_verified: true,
            pq_leg_verified: true,
            ots_leaf_index,
        }
    }

    #[test]
    fn b17_admits_clean_dual_leg_hybrid_and_blocks_ots_leaf_reuse() {
        let mut gate = PostQuantumAgilityGate::default();
        let r1 = gate.admit_envelope(&clean_env(1)).expect("fresh leaf 1");
        assert_eq!(r1.receipt_digest.len(), 64);
        assert_eq!(gate.verified_count(), 1);

        assert_eq!(
            gate.admit_envelope(&clean_env(1)),
            Err(PostQuantumRejection::OtsLeafReuseDetected { ots_leaf_index: 1 })
        );
    }

    #[test]
    fn b17_refuses_suite_downgrade_and_deprecated_suite() {
        let mut gate = PostQuantumAgilityGate::default();
        let mut downgraded = clean_env(2);
        downgraded.suite = CryptoSuite::ClassicalOnlyEd25519;
        assert!(matches!(
            gate.admit_envelope(&downgraded),
            Err(PostQuantumRejection::SuiteDowngradeRefused { .. })
        ));

        gate.deprecate_suite(CryptoSuite::HybridEd25519MlDsa65);
        assert_eq!(
            gate.admit_envelope(&clean_env(3)),
            Err(PostQuantumRejection::DeprecatedSuiteRefused {
                suite: CryptoSuite::HybridEd25519MlDsa65
            })
        );
    }

    #[test]
    fn b17_refuses_single_leg_bypass_in_either_direction() {
        let mut gate = PostQuantumAgilityGate::default();
        let mut missing_pq = clean_env(4);
        missing_pq.pq_leg_verified = false;
        assert!(matches!(
            gate.admit_envelope(&missing_pq),
            Err(PostQuantumRejection::SingleLegBypassRefused {
                classical_leg_verified: true,
                pq_leg_verified: false
            })
        ));

        let mut missing_classical = clean_env(5);
        missing_classical.classical_leg_verified = false;
        assert!(matches!(
            gate.admit_envelope(&missing_classical),
            Err(PostQuantumRejection::SingleLegBypassRefused {
                classical_leg_verified: false,
                pq_leg_verified: true
            })
        ));
    }
}
