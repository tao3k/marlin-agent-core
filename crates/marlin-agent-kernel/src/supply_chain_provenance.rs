// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B14 Hermetic Supply-Chain Provenance, Anti-Slopsquatting & 2-Builder Reproducibility Gate.
//!
//! Enforces the formal invariants of `SupplyChainProvenance.qnt` /
//! `MarlinBackendSupplyChainOverlay.qnt` and `SupplyChainProvenance.lean`:
//!   1. Every declared dependency must exist in the signed registry allowlist (anti-slopsquatting).
//!   2. Every dependency must carry a pinned 64-hex SHA-256 content digest matching the lockfile pin.
//!   3. Any dependency or artifact present in the CVE tombstone registry is deterministically refused.
//!   4. Builds must execute in a hermetic sandbox with zero network egress during compilation.
//!   5. Two distinct independent builders must reproduce identical 64-hex SHA-256 artifact digests.

use std::collections::{BTreeMap, BTreeSet};

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

/// A single pinned dependency entry supplied in a build provenance claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyChainDependencySpec {
    pub package_name: String,
    pub version: String,
    pub sha256_digest: String,
}

/// Complete build provenance claim submitted for B14 release admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyChainBuildClaim {
    pub release_id: String,
    pub artifact_digest: String,
    pub dependencies: Vec<SupplyChainDependencySpec>,
    pub hermetic_sandbox: bool,
    pub network_access_during_build: bool,
    pub builder_a_id: String,
    pub builder_a_digest: String,
    pub builder_b_id: String,
    pub builder_b_digest: String,
}

/// Machine-checkable failure reasons for B14 supply-chain admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupplyChainRejection {
    EmptyDependencies,
    InvalidArtifactDigest { digest: String },
    SlopsquattedOrUnregisteredPackage { package_name: String, version: String },
    UnpinnedOrMismatchedDependencyDigest {
        package_name: String,
        version: String,
        expected: String,
        actual: String,
    },
    CveTombstoneMatched { identifier: String, cve_id: String },
    NonHermeticBuild { reason: String },
    BuilderParityFailed { reason: String },
}

/// Deterministic admission receipt bound to the exact artifact, pinned deps, and builder pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyChainAdmissionReceipt {
    pub release_id: String,
    pub artifact_digest: String,
    pub verified_dependency_count: usize,
    pub builder_a_id: String,
    pub builder_b_id: String,
    pub receipt_digest: String,
}

/// Stateful B14 supply-chain gate holding the signed dependency lockfile allowlist
/// and monotonic CVE tombstone registry.
#[derive(Debug, Clone, Default)]
pub struct SupplyChainProvenanceGate {
    /// `(package_name, version) -> pinned 64-hex SHA-256 digest`.
    allowlist_pins: BTreeMap<(String, String), String>,
    /// Revoked/vulnerable package digest or `package@version` -> CVE identifier.
    cve_tombstones: BTreeMap<String, String>,
    admitted_releases: BTreeSet<String>,
}

impl SupplyChainProvenanceGate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pin an allowlisted `(package_name, version)` to its canonical 64-hex SHA-256 digest.
    pub fn register_pinned_dependency(
        &mut self,
        package_name: impl Into<String>,
        version: impl Into<String>,
        sha256_digest: impl Into<String>,
    ) -> Result<(), SupplyChainRejection> {
        let pkg = package_name.into();
        let ver = version.into();
        let digest = sha256_digest.into().to_ascii_lowercase();
        if !is_valid_sha256_hex(&digest) {
            return Err(SupplyChainRejection::UnpinnedOrMismatchedDependencyDigest {
                package_name: pkg,
                version: ver,
                expected: "<64-hex-sha256>".to_owned(),
                actual: digest,
            });
        }
        self.allowlist_pins.insert((pkg, ver), digest);
        Ok(())
    }

    /// Permanently tombstone a vulnerable digest or `package@version` coordinate.
    pub fn record_cve_tombstone(
        &mut self,
        identifier: impl Into<String>,
        cve_id: impl Into<String>,
    ) {
        self.cve_tombstones
            .insert(identifier.into().to_ascii_lowercase(), cve_id.into());
    }

    /// Evaluate and admit a B14 supply-chain build claim fail-closed.
    pub fn admit_build(
        &mut self,
        claim: &SupplyChainBuildClaim,
    ) -> Result<SupplyChainAdmissionReceipt, SupplyChainRejection> {
        let artifact_digest = claim.artifact_digest.to_ascii_lowercase();
        if !is_valid_sha256_hex(&artifact_digest) {
            return Err(SupplyChainRejection::InvalidArtifactDigest {
                digest: claim.artifact_digest.clone(),
            });
        }
        if let Some(cve_id) = self.cve_tombstones.get(&artifact_digest) {
            return Err(SupplyChainRejection::CveTombstoneMatched {
                identifier: artifact_digest,
                cve_id: cve_id.clone(),
            });
        }
        if claim.dependencies.is_empty() {
            return Err(SupplyChainRejection::EmptyDependencies);
        }

        let mut canonical_deps = Vec::with_capacity(claim.dependencies.len());
        for dep in &claim.dependencies {
            let dep_digest = dep.sha256_digest.to_ascii_lowercase();
            let coord = format!("{}@{}", dep.package_name, dep.version).to_ascii_lowercase();

            if let Some(cve_id) = self.cve_tombstones.get(&coord) {
                return Err(SupplyChainRejection::CveTombstoneMatched {
                    identifier: coord,
                    cve_id: cve_id.clone(),
                });
            }
            if let Some(cve_id) = self.cve_tombstones.get(&dep_digest) {
                return Err(SupplyChainRejection::CveTombstoneMatched {
                    identifier: dep_digest,
                    cve_id: cve_id.clone(),
                });
            }

            let Some(expected_digest) = self
                .allowlist_pins
                .get(&(dep.package_name.clone(), dep.version.clone()))
            else {
                return Err(SupplyChainRejection::SlopsquattedOrUnregisteredPackage {
                    package_name: dep.package_name.clone(),
                    version: dep.version.clone(),
                });
            };
            if !is_valid_sha256_hex(&dep_digest) || &dep_digest != expected_digest {
                return Err(SupplyChainRejection::UnpinnedOrMismatchedDependencyDigest {
                    package_name: dep.package_name.clone(),
                    version: dep.version.clone(),
                    expected: expected_digest.clone(),
                    actual: dep.sha256_digest.clone(),
                });
            }
            canonical_deps.push(format!("{coord}:{dep_digest}"));
        }

        if !claim.hermetic_sandbox {
            return Err(SupplyChainRejection::NonHermeticBuild {
                reason: "hermetic sandbox isolation was not enabled".to_owned(),
            });
        }
        if claim.network_access_during_build {
            return Err(SupplyChainRejection::NonHermeticBuild {
                reason: "network access occurred during build execution".to_owned(),
            });
        }

        if claim.builder_a_id.trim().is_empty() || claim.builder_b_id.trim().is_empty() {
            return Err(SupplyChainRejection::BuilderParityFailed {
                reason: "both independent builder identities must be non-empty".to_owned(),
            });
        }
        if claim.builder_a_id == claim.builder_b_id {
            return Err(SupplyChainRejection::BuilderParityFailed {
                reason: format!(
                    "two distinct builders required; got duplicate builder '{}'",
                    claim.builder_a_id
                ),
            });
        }
        let digest_a = claim.builder_a_digest.to_ascii_lowercase();
        let digest_b = claim.builder_b_digest.to_ascii_lowercase();
        if digest_a != digest_b || digest_a != artifact_digest {
            return Err(SupplyChainRejection::BuilderParityFailed {
                reason: format!(
                    "2-builder artifact divergence: claimed={artifact_digest} builder_a={digest_a} builder_b={digest_b}"
                ),
            });
        }

        canonical_deps.sort();
        let receipt_payload = format!(
            "b14|release={}|artifact={}|builders={},{}|deps={}",
            claim.release_id,
            artifact_digest,
            claim.builder_a_id,
            claim.builder_b_id,
            canonical_deps.join(",")
        );
        let receipt_digest = sha256_hex(receipt_payload.as_bytes());
        self.admitted_releases.insert(claim.release_id.clone());

        Ok(SupplyChainAdmissionReceipt {
            release_id: claim.release_id.clone(),
            artifact_digest,
            verified_dependency_count: canonical_deps.len(),
            builder_a_id: claim.builder_a_id.clone(),
            builder_b_id: claim.builder_b_id.clone(),
            receipt_digest,
        })
    }

    #[must_use]
    pub fn admitted_count(&self) -> usize {
        self.admitted_releases.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn clean_gate_and_claim() -> (SupplyChainProvenanceGate, SupplyChainBuildClaim) {
        let mut gate = SupplyChainProvenanceGate::new();
        gate.register_pinned_dependency("marlin-agent-kernel", "0.4.0", sample_digest('a'))
            .expect("pin kernel");
        gate.register_pinned_dependency("ring", "0.17.14", sample_digest('b'))
            .expect("pin ring");

        let claim = SupplyChainBuildClaim {
            release_id: "rel-2026.11.27".to_owned(),
            artifact_digest: sample_digest('f'),
            dependencies: vec![
                SupplyChainDependencySpec {
                    package_name: "marlin-agent-kernel".to_owned(),
                    version: "0.4.0".to_owned(),
                    sha256_digest: sample_digest('a'),
                },
                SupplyChainDependencySpec {
                    package_name: "ring".to_owned(),
                    version: "0.17.14".to_owned(),
                    sha256_digest: sample_digest('b'),
                },
            ],
            hermetic_sandbox: true,
            network_access_during_build: false,
            builder_a_id: "builder-eu-nix-01".to_owned(),
            builder_a_digest: sample_digest('f'),
            builder_b_id: "builder-us-nix-02".to_owned(),
            builder_b_digest: sample_digest('f'),
        };
        (gate, claim)
    }

    #[test]
    fn b14_admits_clean_hermetic_reproducible_build() {
        let (mut gate, claim) = clean_gate_and_claim();
        let receipt = gate.admit_build(&claim).expect("clean build must admit");
        assert_eq!(receipt.verified_dependency_count, 2);
        assert_eq!(receipt.receipt_digest.len(), 64);
        assert_eq!(gate.admitted_count(), 1);
    }

    #[test]
    fn b14_refuses_slopsquatted_unregistered_dependency() {
        let (mut gate, mut claim) = clean_gate_and_claim();
        claim.dependencies.push(SupplyChainDependencySpec {
            package_name: "marlin-ai-fast-helper-hallucinated".to_owned(),
            version: "1.0.0".to_owned(),
            sha256_digest: sample_digest('c'),
        });
        assert!(matches!(
            gate.admit_build(&claim),
            Err(SupplyChainRejection::SlopsquattedOrUnregisteredPackage { .. })
        ));
    }

    #[test]
    fn b14_refuses_tampered_dependency_digest() {
        let (mut gate, mut claim) = clean_gate_and_claim();
        claim.dependencies[1].sha256_digest = sample_digest('9');
        assert!(matches!(
            gate.admit_build(&claim),
            Err(SupplyChainRejection::UnpinnedOrMismatchedDependencyDigest { .. })
        ));
    }

    #[test]
    fn b14_refuses_cve_tombstoned_dependency_or_artifact() {
        let (mut gate, claim) = clean_gate_and_claim();
        gate.record_cve_tombstone("ring@0.17.14", "CVE-2026-99999");
        assert!(matches!(
            gate.admit_build(&claim),
            Err(SupplyChainRejection::CveTombstoneMatched { .. })
        ));
    }

    #[test]
    fn b14_refuses_non_hermetic_network_build() {
        let (mut gate, mut claim) = clean_gate_and_claim();
        claim.network_access_during_build = true;
        assert!(matches!(
            gate.admit_build(&claim),
            Err(SupplyChainRejection::NonHermeticBuild { .. })
        ));
    }

    #[test]
    fn b14_refuses_single_builder_or_diverged_two_builder_digests() {
        let (mut gate, mut claim) = clean_gate_and_claim();
        claim.builder_b_digest = sample_digest('e');
        assert!(matches!(
            gate.admit_build(&claim),
            Err(SupplyChainRejection::BuilderParityFailed { .. })
        ));

        claim.builder_b_digest = sample_digest('f');
        claim.builder_b_id = claim.builder_a_id.clone();
        assert!(matches!(
            gate.admit_build(&claim),
            Err(SupplyChainRejection::BuilderParityFailed { .. })
        ));
    }
}
