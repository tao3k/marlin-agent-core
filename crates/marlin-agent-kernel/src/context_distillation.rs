// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Rust B12 Controlled Context Distillation & Epistemic Provenance Gate.
//!
//! Refined by:
//! - `engineering.defense.context-distillation-provenance` (`ContextDistillationProvenance.qnt`)
//! - `DeterministicEngineering.Engineering.ContextDistillationAndProvenance` (Lean 4 / LeanPoo)
//! - `dev/quint/threat/MarlinBackendContextDistillationOverlay.qnt`
//!
//! Enforces five fail-closed lattice & provenance invariants on every compacted/distilled
//! agent context window:
//! 1. **Anti-Amnesia**: every pinned `M1` constraint segment MUST be retained verbatim,
//! 2. **Citation Grounding**: every cited `(segment_id, digest)` MUST exist in the source window,
//! 3. **Monotonic Integrity Meet**: distilled integrity <= `min(source.integrity)` (blocks untrusted -> trusted laundering),
//! 4. **Monotonic Secrecy Union**: distilled secrecy >= `max(source.secrecy)` (blocks secrecy stripping via paraphrasing),
//! 5. **Strict Token Budget**: distilled token count <= `max_output_tokens`.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    fmt::Write as _,
};

use crate::governance_admission::{
    GovernanceProofDigest, GovernedSubjectName, governance_sha256_bytes,
};

/// Ordered epistemic integrity lattice (`Untrusted < Validated < PinnedM1`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum EpistemicIntegrity {
    Untrusted = 0,
    Validated = 1,
    PinnedM1 = 2,
}

/// Ordered confidentiality/secrecy lattice (`Public < Internal < Confidential`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SecrecyClass {
    Public = 0,
    Internal = 1,
    Confidential = 2,
}

/// One source segment entering a context compaction/distillation pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextSegment {
    pub segment_id: GovernedSubjectName,
    pub content_digest: GovernanceProofDigest,
    pub integrity: EpistemicIntegrity,
    pub secrecy: SecrecyClass,
    pub token_count: usize,
    pub is_pinned_constraint: bool,
}

/// Candidate distilled/compacted context packet submitted for B12 admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistilledContextPacket {
    pub cited_sources: Vec<(GovernedSubjectName, GovernanceProofDigest)>,
    pub retained_pinned_ids: BTreeSet<GovernedSubjectName>,
    pub output_integrity: EpistemicIntegrity,
    pub output_secrecy: SecrecyClass,
    pub output_tokens: usize,
    pub summary_digest: GovernanceProofDigest,
}

/// Typed, fail-closed rejection reasons for B12 context distillation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextDistillationRejection {
    EmptySourceWindow,
    EmptyCitations,
    PinnedConstraintAmnesia(GovernedSubjectName),
    FabricatedCitation {
        segment_id: GovernedSubjectName,
        cited_digest: GovernanceProofDigest,
    },
    IntegrityLaundering {
        claimed: EpistemicIntegrity,
        required_meet: EpistemicIntegrity,
    },
    SecrecyStripping {
        claimed: SecrecyClass,
        required_union: SecrecyClass,
    },
    TokenBudgetExceeded {
        observed_tokens: usize,
        max_tokens: usize,
    },
}

impl fmt::Display for ContextDistillationRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySourceWindow => write!(f, "distillation source window must not be empty"),
            Self::EmptyCitations => write!(f, "distilled packet must cite at least one source"),
            Self::PinnedConstraintAmnesia(id) => {
                write!(f, "pinned M1 constraint '{id}' was dropped during distillation")
            }
            Self::FabricatedCitation {
                segment_id,
                cited_digest,
            } => write!(
                f,
                "fabricated or mismatched citation for '{segment_id}' ({})",
                cited_digest.as_str()
            ),
            Self::IntegrityLaundering {
                claimed,
                required_meet,
            } => write!(
                f,
                "integrity laundering refused: claimed {claimed:?} exceeds monotonic meet {required_meet:?}"
            ),
            Self::SecrecyStripping {
                claimed,
                required_union,
            } => write!(
                f,
                "secrecy stripping refused: claimed {claimed:?} is below monotonic union {required_union:?}"
            ),
            Self::TokenBudgetExceeded {
                observed_tokens,
                max_tokens,
            } => write!(
                f,
                "distilled token count {observed_tokens} exceeds budget {max_tokens}"
            ),
        }
    }
}

impl Error for ContextDistillationRejection {}

/// Verified B12 distillation provenance receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextDistillationReceipt {
    pub effective_integrity: EpistemicIntegrity,
    pub effective_secrecy: SecrecyClass,
    pub retained_pinned_count: usize,
    pub grounded_citation_count: usize,
    pub output_tokens: usize,
    pub provenance_digest: GovernanceProofDigest,
}

/// Authoritative fail-closed B12 context distillation gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextDistillationGate {
    max_output_tokens: usize,
}

impl ContextDistillationGate {
    pub const fn new(max_output_tokens: usize) -> Self {
        Self { max_output_tokens }
    }

    pub const fn max_output_tokens(&self) -> usize {
        self.max_output_tokens
    }

    pub fn admit_distillation(
        &self,
        sources: &[ContextSegment],
        packet: &DistilledContextPacket,
    ) -> Result<ContextDistillationReceipt, ContextDistillationRejection> {
        if sources.is_empty() {
            return Err(ContextDistillationRejection::EmptySourceWindow);
        }
        if packet.cited_sources.is_empty() {
            return Err(ContextDistillationRejection::EmptyCitations);
        }
        if packet.output_tokens > self.max_output_tokens {
            return Err(ContextDistillationRejection::TokenBudgetExceeded {
                observed_tokens: packet.output_tokens,
                max_tokens: self.max_output_tokens,
            });
        }

        let mut source_index: BTreeMap<&GovernedSubjectName, &ContextSegment> = BTreeMap::new();
        let mut required_meet = EpistemicIntegrity::PinnedM1;
        let mut required_union = SecrecyClass::Public;
        let mut pinned_count = 0usize;

        for seg in sources {
            source_index.insert(&seg.segment_id, seg);
            if seg.integrity < required_meet {
                required_meet = seg.integrity;
            }
            if seg.secrecy > required_union {
                required_union = seg.secrecy;
            }
            if seg.is_pinned_constraint {
                pinned_count += 1;
                if !packet.retained_pinned_ids.contains(&seg.segment_id) {
                    return Err(ContextDistillationRejection::PinnedConstraintAmnesia(
                        seg.segment_id.clone(),
                    ));
                }
            }
        }

        for (seg_id, cited_digest) in &packet.cited_sources {
            match source_index.get(seg_id) {
                Some(seg) if &seg.content_digest == cited_digest => {}
                _ => {
                    return Err(ContextDistillationRejection::FabricatedCitation {
                        segment_id: seg_id.clone(),
                        cited_digest: cited_digest.clone(),
                    });
                }
            }
        }

        if packet.output_integrity > required_meet {
            return Err(ContextDistillationRejection::IntegrityLaundering {
                claimed: packet.output_integrity,
                required_meet,
            });
        }
        if packet.output_secrecy < required_union {
            return Err(ContextDistillationRejection::SecrecyStripping {
                claimed: packet.output_secrecy,
                required_union,
            });
        }

        let mut canonical = Vec::new();
        canonical.extend_from_slice(b"marlin:b12:distill:v1\0");
        canonical.extend_from_slice(packet.summary_digest.as_str().as_bytes());
        canonical.push(packet.output_integrity as u8);
        canonical.push(packet.output_secrecy as u8);
        canonical.extend_from_slice(&(packet.output_tokens as u64).to_be_bytes());
        for (seg_id, digest) in &packet.cited_sources {
            canonical.extend_from_slice(seg_id.as_str().as_bytes());
            canonical.push(b'=');
            canonical.extend_from_slice(digest.as_str().as_bytes());
            canonical.push(b';');
        }
        let raw = governance_sha256_bytes(&canonical);
        let mut hex = String::with_capacity(7 + 64);
        hex.push_str("sha256:");
        for b in raw {
            let _ = write!(&mut hex, "{b:02x}");
        }
        let provenance_digest =
            GovernanceProofDigest::new(hex).expect("32-byte SHA-256 hex always valid");

        Ok(ContextDistillationReceipt {
            effective_integrity: packet.output_integrity,
            effective_secrecy: packet.output_secrecy,
            retained_pinned_count: pinned_count,
            grounded_citation_count: packet.cited_sources.len(),
            output_tokens: packet.output_tokens,
            provenance_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_of(s: &str) -> GovernanceProofDigest {
        let raw = governance_sha256_bytes(s.as_bytes());
        let mut hex = String::with_capacity(7 + 64);
        hex.push_str("sha256:");
        for b in raw {
            let _ = write!(&mut hex, "{b:02x}");
        }
        GovernanceProofDigest::new(hex).unwrap()
    }

    fn id(s: &str) -> GovernedSubjectName {
        GovernedSubjectName::new(s).unwrap()
    }

    fn sample_sources() -> Vec<ContextSegment> {
        vec![
            ContextSegment {
                segment_id: id("pin-policy-sod"),
                content_digest: digest_of("policy:sod-v1"),
                integrity: EpistemicIntegrity::PinnedM1,
                secrecy: SecrecyClass::Internal,
                token_count: 120,
                is_pinned_constraint: true,
            },
            ContextSegment {
                segment_id: id("untrusted-web-doc"),
                content_digest: digest_of("web:snippet-42"),
                integrity: EpistemicIntegrity::Untrusted,
                secrecy: SecrecyClass::Public,
                token_count: 800,
                is_pinned_constraint: false,
            },
            ContextSegment {
                segment_id: id("tenant-secret-note"),
                content_digest: digest_of("secret:tenant-a"),
                integrity: EpistemicIntegrity::Validated,
                secrecy: SecrecyClass::Confidential,
                token_count: 300,
                is_pinned_constraint: false,
            },
        ]
    }

    fn clean_packet(sources: &[ContextSegment]) -> DistilledContextPacket {
        let mut retained = BTreeSet::new();
        retained.insert(id("pin-policy-sod"));
        DistilledContextPacket {
            cited_sources: sources
                .iter()
                .map(|s| (s.segment_id.clone(), s.content_digest.clone()))
                .collect(),
            retained_pinned_ids: retained,
            output_integrity: EpistemicIntegrity::Untrusted,
            output_secrecy: SecrecyClass::Confidential,
            output_tokens: 256,
            summary_digest: digest_of("distilled-summary-v1"),
        }
    }

    #[test]
    fn admits_safe_distillation_with_monotonic_meet_and_secrecy_union() {
        let gate = ContextDistillationGate::new(512);
        let sources = sample_sources();
        let packet = clean_packet(&sources);
        let receipt = gate
            .admit_distillation(&sources, &packet)
            .expect("safe distillation admitted");
        assert_eq!(receipt.effective_integrity, EpistemicIntegrity::Untrusted);
        assert_eq!(receipt.effective_secrecy, SecrecyClass::Confidential);
        assert_eq!(receipt.retained_pinned_count, 1);
        assert_eq!(receipt.grounded_citation_count, 3);
    }

    #[test]
    fn refuses_all_five_b12_distillation_attacks_fail_closed() {
        let gate = ContextDistillationGate::new(512);
        let sources = sample_sources();
        let base = clean_packet(&sources);

        // 1. Pinned M1 constraint amnesia
        let mut amnesia = base.clone();
        amnesia.retained_pinned_ids.clear();
        assert_eq!(
            gate.admit_distillation(&sources, &amnesia),
            Err(ContextDistillationRejection::PinnedConstraintAmnesia(id(
                "pin-policy-sod"
            )))
        );

        // 2. Fabricated citation digest
        let mut fabricated = base.clone();
        fabricated
            .cited_sources
            .push((id("ghost-source"), digest_of("hallucinated")));
        assert!(matches!(
            gate.admit_distillation(&sources, &fabricated),
            Err(ContextDistillationRejection::FabricatedCitation { .. })
        ));

        // 3. Integrity laundering (Untrusted -> Validated)
        let mut laundered = base.clone();
        laundered.output_integrity = EpistemicIntegrity::Validated;
        assert_eq!(
            gate.admit_distillation(&sources, &laundered),
            Err(ContextDistillationRejection::IntegrityLaundering {
                claimed: EpistemicIntegrity::Validated,
                required_meet: EpistemicIntegrity::Untrusted,
            })
        );

        // 4. Secrecy stripping via paraphrasing (Confidential -> Public)
        let mut stripped = base.clone();
        stripped.output_secrecy = SecrecyClass::Public;
        assert_eq!(
            gate.admit_distillation(&sources, &stripped),
            Err(ContextDistillationRejection::SecrecyStripping {
                claimed: SecrecyClass::Public,
                required_union: SecrecyClass::Confidential,
            })
        );

        // 5. Token budget overflow
        let mut overflow = base;
        overflow.output_tokens = 1024;
        assert_eq!(
            gate.admit_distillation(&sources, &overflow),
            Err(ContextDistillationRejection::TokenBudgetExceeded {
                observed_tokens: 1024,
                max_tokens: 512,
            })
        );
    }
}
