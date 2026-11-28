// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B19 Cross-Region Sovereign Data-Residency & Geo-Fencing Gate.
//!
//! Enforces the formal invariants of `GeoResidencySovereignty.qnt` /
//! `MarlinBackendGeoResidencyOverlay.qnt` and `GeoResidencyAndSovereignty.lean`:
//!   1. Storage target region must belong to the allowed jurisdiction set and never be sanctioned.
//!   2. LLM inference execution region must independently satisfy residency & sanction checks
//!      (blocks hidden model-inference data escape across borders).
//!   3. Intra-region placement (`source == target == inference`) requires zero cross-border permit
//!      (anti-P4 operational sufficiency).
//!   4. Any cross-border transfer requires a valid, non-revoked, non-expired `CrossBorderPermit`
//!      issued by an unrevoked sovereign authority matching the exact tenant and region pair.

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

fn norm_region(r: &str) -> String {
    r.trim().to_ascii_uppercase()
}

/// Signed cross-border transfer permit required whenever data or inference crosses jurisdictions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossBorderPermit {
    pub permit_id: String,
    pub authority_id: String,
    pub tenant_id: String,
    pub from_region: String,
    pub to_region: String,
    pub expires_at_unix_ms: u64,
    pub permit_digest: String,
}

/// Placement & inference residency claim submitted for B19 admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoPlacementClaim {
    pub placement_id: String,
    pub tenant_id: String,
    pub source_region: String,
    pub target_storage_region: String,
    pub inference_region: String,
    pub observed_unix_ms: u64,
    pub cross_border_permit: Option<CrossBorderPermit>,
}

/// Machine-checkable refusal reasons for B19 sovereign geo-residency enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeoResidencyRejection {
    SanctionedTargetRegion {
        region: String,
    },
    DisallowedTargetRegion {
        region: String,
    },
    SanctionedInferenceRegion {
        region: String,
    },
    InferenceRegionEscape {
        region: String,
    },
    MissingCrossBorderPermit {
        from_region: String,
        destination_region: String,
    },
    InvalidCrossBorderPermit {
        reason: String,
    },
    RevokedPermitOrAuthority {
        identifier: String,
    },
    ExpiredCrossBorderPermit {
        permit_id: String,
        expires_at_unix_ms: u64,
        observed_unix_ms: u64,
    },
}

/// Deterministic admission receipt bound to regions, tenant, and optional permit digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoPlacementReceipt {
    pub placement_id: String,
    pub tenant_id: String,
    pub target_storage_region: String,
    pub inference_region: String,
    pub cross_border: bool,
    pub receipt_digest: String,
}

/// Sovereign geo-residency policy & revocation registry.
#[derive(Debug, Clone, Default)]
pub struct GeoResidencySovereigntyGate {
    allowed_storage_regions: BTreeSet<String>,
    allowed_inference_regions: BTreeSet<String>,
    sanctioned_regions: BTreeSet<String>,
    trusted_authorities: BTreeSet<String>,
    revoked_authorities: BTreeSet<String>,
    revoked_permits: BTreeSet<String>,
    admitted_count: u64,
}

impl GeoResidencySovereigntyGate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_storage_region(&mut self, region: impl AsRef<str>) {
        self.allowed_storage_regions
            .insert(norm_region(region.as_ref()));
    }

    pub fn allow_inference_region(&mut self, region: impl AsRef<str>) {
        self.allowed_inference_regions
            .insert(norm_region(region.as_ref()));
    }

    pub fn sanction_region(&mut self, region: impl AsRef<str>) {
        self.sanctioned_regions.insert(norm_region(region.as_ref()));
    }

    pub fn trust_authority(&mut self, authority_id: impl Into<String>) {
        self.trusted_authorities.insert(authority_id.into());
    }

    pub fn revoke_authority(&mut self, authority_id: impl Into<String>) {
        self.revoked_authorities.insert(authority_id.into());
    }

    pub fn revoke_permit(&mut self, permit_id: impl Into<String>) {
        self.revoked_permits.insert(permit_id.into());
    }

    #[must_use]
    pub fn admitted_count(&self) -> u64 {
        self.admitted_count
    }

    /// Evaluate and admit a placement & inference claim fail-closed.
    pub fn admit_placement(
        &mut self,
        claim: &GeoPlacementClaim,
    ) -> Result<GeoPlacementReceipt, GeoResidencyRejection> {
        let source = norm_region(&claim.source_region);
        let target = norm_region(&claim.target_storage_region);
        let inference = norm_region(&claim.inference_region);

        if self.sanctioned_regions.contains(&target) {
            return Err(GeoResidencyRejection::SanctionedTargetRegion { region: target });
        }
        if !self.allowed_storage_regions.contains(&target) {
            return Err(GeoResidencyRejection::DisallowedTargetRegion { region: target });
        }
        if self.sanctioned_regions.contains(&inference) {
            return Err(GeoResidencyRejection::SanctionedInferenceRegion { region: inference });
        }
        if !self.allowed_inference_regions.contains(&inference) {
            return Err(GeoResidencyRejection::InferenceRegionEscape { region: inference });
        }

        let cross_border_storage = source != target;
        let cross_border_inference = source != inference;
        let cross_border = cross_border_storage || cross_border_inference;
        let permit_digest_str = if cross_border {
            let destination = if cross_border_inference {
                inference.clone()
            } else {
                target.clone()
            };
            let Some(permit) = claim.cross_border_permit.as_ref() else {
                return Err(GeoResidencyRejection::MissingCrossBorderPermit {
                    from_region: source,
                    destination_region: destination,
                });
            };
            if self.revoked_permits.contains(&permit.permit_id) {
                return Err(GeoResidencyRejection::RevokedPermitOrAuthority {
                    identifier: permit.permit_id.clone(),
                });
            }
            if self.revoked_authorities.contains(&permit.authority_id) {
                return Err(GeoResidencyRejection::RevokedPermitOrAuthority {
                    identifier: permit.authority_id.clone(),
                });
            }
            if !self.trusted_authorities.contains(&permit.authority_id) {
                return Err(GeoResidencyRejection::InvalidCrossBorderPermit {
                    reason: format!("untrusted transfer authority '{}'", permit.authority_id),
                });
            }
            if permit.tenant_id != claim.tenant_id {
                return Err(GeoResidencyRejection::InvalidCrossBorderPermit {
                    reason: format!(
                        "permit tenant '{}' does not match claim tenant '{}'",
                        permit.tenant_id, claim.tenant_id
                    ),
                });
            }
            let p_from = norm_region(&permit.from_region);
            let p_to = norm_region(&permit.to_region);
            if p_from != source
                || (cross_border_storage && p_to != target)
                || (cross_border_inference && p_to != inference)
            {
                return Err(GeoResidencyRejection::InvalidCrossBorderPermit {
                    reason: format!(
                        "permit corridor {p_from}->{p_to} does not cover {source}->({target},{inference})"
                    ),
                });
            }
            if claim.observed_unix_ms > permit.expires_at_unix_ms {
                return Err(GeoResidencyRejection::ExpiredCrossBorderPermit {
                    permit_id: permit.permit_id.clone(),
                    expires_at_unix_ms: permit.expires_at_unix_ms,
                    observed_unix_ms: claim.observed_unix_ms,
                });
            }
            let digest = permit.permit_digest.to_ascii_lowercase();
            if !is_valid_sha256_hex(&digest) {
                return Err(GeoResidencyRejection::InvalidCrossBorderPermit {
                    reason: "permit digest must be 64-hex SHA-256".to_owned(),
                });
            }
            digest
        } else {
            "intra-region".to_owned()
        };

        self.admitted_count = self.admitted_count.saturating_add(1);
        let receipt_digest = sha256_hex(
            format!(
                "b19|id={}|tenant={}|src={source}|dst={target}|inf={inference}|permit={permit_digest_str}",
                claim.placement_id, claim.tenant_id
            )
            .as_bytes(),
        );
        Ok(GeoPlacementReceipt {
            placement_id: claim.placement_id.clone(),
            tenant_id: claim.tenant_id.clone(),
            target_storage_region: target,
            inference_region: inference,
            cross_border,
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

    fn base_gate() -> GeoResidencySovereigntyGate {
        let mut gate = GeoResidencySovereigntyGate::new();
        gate.allow_storage_region("EU-DE");
        gate.allow_storage_region("EU-IE");
        gate.allow_inference_region("EU-DE");
        gate.allow_inference_region("EU-IE");
        gate.sanction_region("XX-SANCTIONED");
        gate.trust_authority("eu-dpa-authority-1");
        gate
    }

    #[test]
    fn b19_admits_intra_region_without_permit_and_permitted_cross_border() {
        let mut gate = base_gate();
        let local = GeoPlacementClaim {
            placement_id: "pl-local".to_owned(),
            tenant_id: "tenant-eu".to_owned(),
            source_region: "EU-DE".to_owned(),
            target_storage_region: "EU-DE".to_owned(),
            inference_region: "EU-DE".to_owned(),
            observed_unix_ms: 1_700_000_000_000,
            cross_border_permit: None,
        };
        let r_local = gate
            .admit_placement(&local)
            .expect("intra-region admits without permit");
        assert!(!r_local.cross_border);

        let xborder = GeoPlacementClaim {
            placement_id: "pl-xborder".to_owned(),
            tenant_id: "tenant-eu".to_owned(),
            source_region: "EU-DE".to_owned(),
            target_storage_region: "EU-IE".to_owned(),
            inference_region: "EU-IE".to_owned(),
            observed_unix_ms: 1_700_000_000_000,
            cross_border_permit: Some(CrossBorderPermit {
                permit_id: "perm-01".to_owned(),
                authority_id: "eu-dpa-authority-1".to_owned(),
                tenant_id: "tenant-eu".to_owned(),
                from_region: "EU-DE".to_owned(),
                to_region: "EU-IE".to_owned(),
                expires_at_unix_ms: 1_800_000_000_000,
                permit_digest: hex64('c'),
            }),
        };
        let r_xb = gate
            .admit_placement(&xborder)
            .expect("permitted cross-border admits");
        assert!(r_xb.cross_border);
        assert_eq!(gate.admitted_count(), 2);
    }

    #[test]
    fn b19_refuses_sanctioned_target_and_hidden_inference_escape() {
        let mut gate = base_gate();
        let sanctioned = GeoPlacementClaim {
            placement_id: "pl-sanctioned".to_owned(),
            tenant_id: "tenant-eu".to_owned(),
            source_region: "EU-DE".to_owned(),
            target_storage_region: "XX-SANCTIONED".to_owned(),
            inference_region: "EU-DE".to_owned(),
            observed_unix_ms: 1_700_000_000_000,
            cross_border_permit: None,
        };
        assert!(matches!(
            gate.admit_placement(&sanctioned),
            Err(GeoResidencyRejection::SanctionedTargetRegion { .. })
        ));

        let inf_escape = GeoPlacementClaim {
            placement_id: "pl-escape".to_owned(),
            tenant_id: "tenant-eu".to_owned(),
            source_region: "EU-DE".to_owned(),
            target_storage_region: "EU-DE".to_owned(),
            inference_region: "US-EAST-1".to_owned(),
            observed_unix_ms: 1_700_000_000_000,
            cross_border_permit: None,
        };
        assert!(matches!(
            gate.admit_placement(&inf_escape),
            Err(GeoResidencyRejection::InferenceRegionEscape { .. })
        ));
    }

    #[test]
    fn b19_refuses_unpermitted_or_revoked_cross_border_transfer() {
        let mut gate = base_gate();
        let unpermitted = GeoPlacementClaim {
            placement_id: "pl-no-perm".to_owned(),
            tenant_id: "tenant-eu".to_owned(),
            source_region: "EU-DE".to_owned(),
            target_storage_region: "EU-IE".to_owned(),
            inference_region: "EU-IE".to_owned(),
            observed_unix_ms: 1_700_000_000_000,
            cross_border_permit: None,
        };
        assert!(matches!(
            gate.admit_placement(&unpermitted),
            Err(GeoResidencyRejection::MissingCrossBorderPermit { .. })
        ));

        gate.revoke_authority("eu-dpa-authority-1");
        let revoked = GeoPlacementClaim {
            cross_border_permit: Some(CrossBorderPermit {
                permit_id: "perm-02".to_owned(),
                authority_id: "eu-dpa-authority-1".to_owned(),
                tenant_id: "tenant-eu".to_owned(),
                from_region: "EU-DE".to_owned(),
                to_region: "EU-IE".to_owned(),
                expires_at_unix_ms: 1_800_000_000_000,
                permit_digest: hex64('d'),
            }),
            ..unpermitted
        };
        assert!(matches!(
            gate.admit_placement(&revoked),
            Err(GeoResidencyRejection::RevokedPermitOrAuthority { .. })
        ));
    }
}
