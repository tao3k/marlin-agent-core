// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B20 Multi-Org Federated Reciprocity & Dispute Arbitration Gate.
//!
//! Enforces the formal invariants of `FederatedOrgReciprocity.qnt` /
//! `MarlinBackendFederatedOrgOverlay.qnt` and `FederatedOrgReciprocity.lean`:
//!   1. Federated operations require a ratified, non-expired bilateral treaty binding
//!      two distinct member organizations (blocks self-reciprocity spoofing).
//!   2. Both origin and target organization cryptographic countersignatures must verify
//!      (blocks unilateral cross-org execution).
//!   3. Any active dispute freeze halts federated operations immediately until explicitly resolved.

use std::collections::BTreeMap;

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

/// Bilateral reciprocity treaty registered between two distinct organizations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilateralTreaty {
    pub treaty_id: String,
    pub org_a: String,
    pub org_b: String,
    pub treaty_epoch: u64,
    pub ratified: bool,
    pub expires_at_unix_ms: u64,
    pub dispute_freeze_active: bool,
}

/// Cross-organization federated operation claim submitted for B20 admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedOpClaim {
    pub op_id: String,
    pub treaty_id: String,
    pub origin_org: String,
    pub target_org: String,
    pub origin_signature_verified: bool,
    pub target_signature_verified: bool,
    pub observed_unix_ms: u64,
    pub payload_digest: String,
}

/// Machine-checkable refusal reasons for B20 federated reciprocity enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FederatedOrgRejection {
    SelfReciprocitySpoofing { org: String },
    UnknownTreaty { treaty_id: String },
    TreatyNotRatified { treaty_id: String },
    TreatyExpired { treaty_id: String, expires_at_unix_ms: u64, observed_unix_ms: u64 },
    OrgPairMismatch { treaty_id: String, origin_org: String, target_org: String },
    UnilateralSignatureRefused { origin_verified: bool, target_verified: bool },
    DisputeFreezeActive { treaty_id: String },
    InvalidPayloadDigest { digest: String },
}

/// Deterministic admission receipt for a ratified, bilaterally countersigned federated op.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedOpReceipt {
    pub op_id: String,
    pub treaty_id: String,
    pub treaty_epoch: u64,
    pub origin_org: String,
    pub target_org: String,
    pub receipt_digest: String,
}

/// Stateful B20 multi-org reciprocity and dispute-arbitration gate.
#[derive(Debug, Clone, Default)]
pub struct FederatedOrgReciprocityGate {
    treaties: BTreeMap<String, BilateralTreaty>,
    admitted_ops: u64,
}

impl FederatedOrgReciprocityGate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a bilateral treaty between two distinct organizations.
    pub fn register_treaty(&mut self, treaty: BilateralTreaty) -> Result<(), FederatedOrgRejection> {
        if treaty.org_a.trim().is_empty()
            || treaty.org_b.trim().is_empty()
            || treaty.org_a == treaty.org_b
        {
            return Err(FederatedOrgRejection::SelfReciprocitySpoofing {
                org: treaty.org_a,
            });
        }
        self.treaties.insert(treaty.treaty_id.clone(), treaty);
        Ok(())
    }

    /// Activate an immediate dispute freeze on a bilateral treaty.
    pub fn raise_dispute_freeze(&mut self, treaty_id: &str) -> Result<(), FederatedOrgRejection> {
        let Some(treaty) = self.treaties.get_mut(treaty_id) else {
            return Err(FederatedOrgRejection::UnknownTreaty {
                treaty_id: treaty_id.to_owned(),
            });
        };
        treaty.dispute_freeze_active = true;
        Ok(())
    }

    /// Lift an active dispute freeze after bilateral arbitration concludes.
    pub fn resolve_dispute_freeze(&mut self, treaty_id: &str) -> Result<(), FederatedOrgRejection> {
        let Some(treaty) = self.treaties.get_mut(treaty_id) else {
            return Err(FederatedOrgRejection::UnknownTreaty {
                treaty_id: treaty_id.to_owned(),
            });
        };
        treaty.dispute_freeze_active = false;
        Ok(())
    }

    #[must_use]
    pub fn admitted_ops(&self) -> u64 {
        self.admitted_ops
    }

    /// Admit a federated operation strictly under a ratified, active, bilaterally signed treaty.
    pub fn admit_federated_op(
        &mut self,
        claim: &FederatedOpClaim,
    ) -> Result<FederatedOpReceipt, FederatedOrgRejection> {
        if claim.origin_org == claim.target_org {
            return Err(FederatedOrgRejection::SelfReciprocitySpoofing {
                org: claim.origin_org.clone(),
            });
        }
        let payload = claim.payload_digest.to_ascii_lowercase();
        if !is_valid_sha256_hex(&payload) {
            return Err(FederatedOrgRejection::InvalidPayloadDigest {
                digest: claim.payload_digest.clone(),
            });
        }
        let Some(treaty) = self.treaties.get(&claim.treaty_id) else {
            return Err(FederatedOrgRejection::UnknownTreaty {
                treaty_id: claim.treaty_id.clone(),
            });
        };
        if !treaty.ratified {
            return Err(FederatedOrgRejection::TreatyNotRatified {
                treaty_id: treaty.treaty_id.clone(),
            });
        }
        if claim.observed_unix_ms > treaty.expires_at_unix_ms {
            return Err(FederatedOrgRejection::TreatyExpired {
                treaty_id: treaty.treaty_id.clone(),
                expires_at_unix_ms: treaty.expires_at_unix_ms,
                observed_unix_ms: claim.observed_unix_ms,
            });
        }
        let pair_matches = (claim.origin_org == treaty.org_a && claim.target_org == treaty.org_b)
            || (claim.origin_org == treaty.org_b && claim.target_org == treaty.org_a);
        if !pair_matches {
            return Err(FederatedOrgRejection::OrgPairMismatch {
                treaty_id: treaty.treaty_id.clone(),
                origin_org: claim.origin_org.clone(),
                target_org: claim.target_org.clone(),
            });
        }
        if treaty.dispute_freeze_active {
            return Err(FederatedOrgRejection::DisputeFreezeActive {
                treaty_id: treaty.treaty_id.clone(),
            });
        }
        if !claim.origin_signature_verified || !claim.target_signature_verified {
            return Err(FederatedOrgRejection::UnilateralSignatureRefused {
                origin_verified: claim.origin_signature_verified,
                target_verified: claim.target_signature_verified,
            });
        }

        self.admitted_ops = self.admitted_ops.saturating_add(1);
        let receipt_digest = sha256_hex(
            format!(
                "b20|op={}|treaty={}|epoch={}|{}->{}|payload={payload}",
                claim.op_id, treaty.treaty_id, treaty.treaty_epoch, claim.origin_org, claim.target_org
            )
            .as_bytes(),
        );
        Ok(FederatedOpReceipt {
            op_id: claim.op_id.clone(),
            treaty_id: treaty.treaty_id.clone(),
            treaty_epoch: treaty.treaty_epoch,
            origin_org: claim.origin_org.clone(),
            target_org: claim.target_org.clone(),
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

    fn setup_gate_and_claim() -> (FederatedOrgReciprocityGate, FederatedOpClaim) {
        let mut gate = FederatedOrgReciprocityGate::new();
        gate.register_treaty(BilateralTreaty {
            treaty_id: "treaty-alpha-beta-v1".to_owned(),
            org_a: "org-alpha".to_owned(),
            org_b: "org-beta".to_owned(),
            treaty_epoch: 1,
            ratified: true,
            expires_at_unix_ms: 1_900_000_000_000,
            dispute_freeze_active: false,
        })
        .expect("register treaty");
        let claim = FederatedOpClaim {
            op_id: "fed-op-01".to_owned(),
            treaty_id: "treaty-alpha-beta-v1".to_owned(),
            origin_org: "org-alpha".to_owned(),
            target_org: "org-beta".to_owned(),
            origin_signature_verified: true,
            target_signature_verified: true,
            observed_unix_ms: 1_700_000_000_000,
            payload_digest: hex64('7'),
        };
        (gate, claim)
    }

    #[test]
    fn b20_admits_bilateral_op_and_enforces_dispute_freeze_and_recovery() {
        let (mut gate, claim) = setup_gate_and_claim();
        let r1 = gate.admit_federated_op(&claim).expect("admit bilateral");
        assert_eq!(r1.receipt_digest.len(), 64);

        gate.raise_dispute_freeze("treaty-alpha-beta-v1").expect("freeze");
        assert!(matches!(
            gate.admit_federated_op(&claim),
            Err(FederatedOrgRejection::DisputeFreezeActive { .. })
        ));

        gate.resolve_dispute_freeze("treaty-alpha-beta-v1").expect("resolve");
        gate.admit_federated_op(&claim).expect("admit after arbitration");
        assert_eq!(gate.admitted_ops(), 2);
    }

    #[test]
    fn b20_refuses_unilateral_unratified_and_self_spoofing() {
        let (mut gate, mut claim) = setup_gate_and_claim();
        claim.target_signature_verified = false;
        assert!(matches!(
            gate.admit_federated_op(&claim),
            Err(FederatedOrgRejection::UnilateralSignatureRefused { .. })
        ));

        claim.target_signature_verified = true;
        claim.target_org = "org-alpha".to_owned();
        assert!(matches!(
            gate.admit_federated_op(&claim),
            Err(FederatedOrgRejection::SelfReciprocitySpoofing { .. })
        ));

        gate.register_treaty(BilateralTreaty {
            treaty_id: "treaty-draft".to_owned(),
            org_a: "org-alpha".to_owned(),
            org_b: "org-gamma".to_owned(),
            treaty_epoch: 1,
            ratified: false,
            expires_at_unix_ms: 1_900_000_000_000,
            dispute_freeze_active: false,
        })
        .expect("register draft");
        claim.treaty_id = "treaty-draft".to_owned();
        claim.target_org = "org-gamma".to_owned();
        assert!(matches!(
            gate.admit_federated_op(&claim),
            Err(FederatedOrgRejection::TreatyNotRatified { .. })
        ));
    }
}
