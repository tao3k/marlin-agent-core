// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B16 Zero-Downtime Key Rotation & Irreversible Cryptographic Shredding Gate.
//!
//! Enforces the formal invariants of `KeyRotationCryptoShredding.qnt` /
//! `MarlinBackendKeyRotationOverlay.qnt` and `KeyRotationAndCryptoShredding.lean`:
//!   1. Key epochs advance strictly monotonically (`next_epoch == active_epoch + 1`).
//!   2. New writes must target `active_epoch` (refusing stale/retired-epoch writes).
//!   3. Shredding a key epoch records an irreversible tombstone blocking future writes,
//!      reads, rewraps, or epoch resurrection.
//!   4. Ciphertext rewrap across epochs must preserve the exact 64-hex SHA-256 plaintext
//!      digest and enforce tenant AAD isolation.

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

/// Cryptographic write claim against the active key epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoWriteClaim {
    pub tenant_id: String,
    pub write_epoch: u64,
    pub plaintext_digest: String,
}

/// Ciphertext rewrap claim migrating a record from an older live epoch to `active_epoch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoRewrapClaim {
    pub tenant_id: String,
    pub from_epoch: u64,
    pub to_epoch: u64,
    pub before_plaintext_digest: String,
    pub after_plaintext_digest: String,
}

/// Machine-checkable refusal reasons for B16 key rotation & crypto-shredding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRotationRejection {
    TenantMismatch {
        expected: String,
        actual: String,
    },
    NonMonotonicEpochRotation {
        active_epoch: u64,
        requested_epoch: u64,
    },
    InvalidDigest {
        digest: String,
    },
    StaleWriteEpoch {
        active_epoch: u64,
        write_epoch: u64,
    },
    EpochShredded {
        epoch: u64,
        tombstone_digest: String,
    },
    ActiveEpochCannotBeShreddedWithoutRotation {
        active_epoch: u64,
    },
    PlaintextDigestMutatedDuringRewrap {
        before: String,
        after: String,
    },
    InvalidRewrapEpochs {
        from_epoch: u64,
        to_epoch: u64,
        active_epoch: u64,
    },
}

/// Deterministic receipt emitted on monotonic epoch rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRotationReceipt {
    pub tenant_id: String,
    pub previous_epoch: u64,
    pub active_epoch: u64,
    pub key_commitment_digest: String,
}

/// Irreversible cryptographic shredding tombstone for a retired key epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoShredTombstone {
    pub tenant_id: String,
    pub shredded_epoch: u64,
    pub reason: String,
    pub tombstone_digest: String,
}

/// Deterministic receipt emitted on admitted write or rewrap operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoOpReceipt {
    pub tenant_id: String,
    pub epoch: u64,
    pub plaintext_digest: String,
    pub receipt_digest: String,
}

/// Per-tenant B16 zero-downtime key rotation and crypto-shredding state machine.
#[derive(Debug, Clone)]
pub struct KeyRotationShreddingGate {
    tenant_id: String,
    active_epoch: u64,
    epoch_commitments: BTreeMap<u64, String>,
    shredded_epochs: BTreeMap<u64, CryptoShredTombstone>,
    admitted_ops: u64,
}

impl KeyRotationShreddingGate {
    /// Initialize a tenant key gate at epoch 1 with its initial 64-hex key commitment digest.
    pub fn new(
        tenant_id: impl Into<String>,
        initial_key_commitment: impl Into<String>,
    ) -> Result<Self, KeyRotationRejection> {
        let tenant_id = tenant_id.into();
        let commitment = initial_key_commitment.into().to_ascii_lowercase();
        if !is_valid_sha256_hex(&commitment) {
            return Err(KeyRotationRejection::InvalidDigest { digest: commitment });
        }
        let mut epoch_commitments = BTreeMap::new();
        epoch_commitments.insert(1, commitment);
        Ok(Self {
            tenant_id,
            active_epoch: 1,
            epoch_commitments,
            shredded_epochs: BTreeMap::new(),
            admitted_ops: 0,
        })
    }

    #[must_use]
    pub fn active_epoch(&self) -> u64 {
        self.active_epoch
    }

    #[must_use]
    pub fn is_epoch_shredded(&self, epoch: u64) -> bool {
        self.shredded_epochs.contains_key(&epoch)
    }

    #[must_use]
    pub fn admitted_ops(&self) -> u64 {
        self.admitted_ops
    }

    /// Advance the active write epoch strictly by +1 (zero-downtime rotation).
    pub fn rotate_to_epoch(
        &mut self,
        next_epoch: u64,
        next_key_commitment: impl Into<String>,
    ) -> Result<KeyRotationReceipt, KeyRotationRejection> {
        let expected = self.active_epoch.saturating_add(1);
        if next_epoch != expected {
            return Err(KeyRotationRejection::NonMonotonicEpochRotation {
                active_epoch: self.active_epoch,
                requested_epoch: next_epoch,
            });
        }
        if let Some(tombstone) = self.shredded_epochs.get(&next_epoch) {
            return Err(KeyRotationRejection::EpochShredded {
                epoch: next_epoch,
                tombstone_digest: tombstone.tombstone_digest.clone(),
            });
        }
        let commitment = next_key_commitment.into().to_ascii_lowercase();
        if !is_valid_sha256_hex(&commitment) {
            return Err(KeyRotationRejection::InvalidDigest { digest: commitment });
        }
        let previous_epoch = self.active_epoch;
        self.active_epoch = next_epoch;
        self.epoch_commitments
            .insert(next_epoch, commitment.clone());
        Ok(KeyRotationReceipt {
            tenant_id: self.tenant_id.clone(),
            previous_epoch,
            active_epoch: next_epoch,
            key_commitment_digest: commitment,
        })
    }

    /// Permanently shred a retired key epoch (`epoch < active_epoch`).
    pub fn shred_retired_epoch(
        &mut self,
        epoch: u64,
        reason: impl Into<String>,
    ) -> Result<CryptoShredTombstone, KeyRotationRejection> {
        if epoch == self.active_epoch {
            return Err(
                KeyRotationRejection::ActiveEpochCannotBeShreddedWithoutRotation {
                    active_epoch: self.active_epoch,
                },
            );
        }
        if let Some(existing) = self.shredded_epochs.get(&epoch) {
            return Ok(existing.clone());
        }
        let reason = reason.into();
        let tombstone_digest = sha256_hex(
            format!(
                "b16-shred|tenant={}|epoch={epoch}|reason={reason}",
                self.tenant_id
            )
            .as_bytes(),
        );
        let tombstone = CryptoShredTombstone {
            tenant_id: self.tenant_id.clone(),
            shredded_epoch: epoch,
            reason,
            tombstone_digest,
        };
        self.epoch_commitments.remove(&epoch);
        self.shredded_epochs.insert(epoch, tombstone.clone());
        Ok(tombstone)
    }

    /// Admit a write strictly against the live `active_epoch` with valid tenant AAD and digest.
    pub fn admit_write(
        &mut self,
        claim: &CryptoWriteClaim,
    ) -> Result<CryptoOpReceipt, KeyRotationRejection> {
        if claim.tenant_id != self.tenant_id {
            return Err(KeyRotationRejection::TenantMismatch {
                expected: self.tenant_id.clone(),
                actual: claim.tenant_id.clone(),
            });
        }
        if let Some(tombstone) = self.shredded_epochs.get(&claim.write_epoch) {
            return Err(KeyRotationRejection::EpochShredded {
                epoch: claim.write_epoch,
                tombstone_digest: tombstone.tombstone_digest.clone(),
            });
        }
        if claim.write_epoch != self.active_epoch {
            return Err(KeyRotationRejection::StaleWriteEpoch {
                active_epoch: self.active_epoch,
                write_epoch: claim.write_epoch,
            });
        }
        let digest = claim.plaintext_digest.to_ascii_lowercase();
        if !is_valid_sha256_hex(&digest) {
            return Err(KeyRotationRejection::InvalidDigest {
                digest: claim.plaintext_digest.clone(),
            });
        }
        self.admitted_ops = self.admitted_ops.saturating_add(1);
        let receipt_digest = sha256_hex(
            format!(
                "b16-write|tenant={}|epoch={}|plaintext={digest}|seq={}",
                self.tenant_id, claim.write_epoch, self.admitted_ops
            )
            .as_bytes(),
        );
        Ok(CryptoOpReceipt {
            tenant_id: self.tenant_id.clone(),
            epoch: claim.write_epoch,
            plaintext_digest: digest,
            receipt_digest,
        })
    }

    /// Admit a zero-downtime rewrap from a live older epoch to `active_epoch`, verifying
    /// exact plaintext SHA-256 continuity.
    pub fn admit_rewrap(
        &mut self,
        claim: &CryptoRewrapClaim,
    ) -> Result<CryptoOpReceipt, KeyRotationRejection> {
        if claim.tenant_id != self.tenant_id {
            return Err(KeyRotationRejection::TenantMismatch {
                expected: self.tenant_id.clone(),
                actual: claim.tenant_id.clone(),
            });
        }
        if let Some(tombstone) = self.shredded_epochs.get(&claim.from_epoch) {
            return Err(KeyRotationRejection::EpochShredded {
                epoch: claim.from_epoch,
                tombstone_digest: tombstone.tombstone_digest.clone(),
            });
        }
        if let Some(tombstone) = self.shredded_epochs.get(&claim.to_epoch) {
            return Err(KeyRotationRejection::EpochShredded {
                epoch: claim.to_epoch,
                tombstone_digest: tombstone.tombstone_digest.clone(),
            });
        }
        if claim.from_epoch >= claim.to_epoch || claim.to_epoch != self.active_epoch {
            return Err(KeyRotationRejection::InvalidRewrapEpochs {
                from_epoch: claim.from_epoch,
                to_epoch: claim.to_epoch,
                active_epoch: self.active_epoch,
            });
        }
        let before = claim.before_plaintext_digest.to_ascii_lowercase();
        let after = claim.after_plaintext_digest.to_ascii_lowercase();
        if !is_valid_sha256_hex(&before) {
            return Err(KeyRotationRejection::InvalidDigest {
                digest: claim.before_plaintext_digest.clone(),
            });
        }
        if !is_valid_sha256_hex(&after) {
            return Err(KeyRotationRejection::InvalidDigest {
                digest: claim.after_plaintext_digest.clone(),
            });
        }
        if before != after {
            return Err(KeyRotationRejection::PlaintextDigestMutatedDuringRewrap { before, after });
        }
        self.admitted_ops = self.admitted_ops.saturating_add(1);
        let receipt_digest = sha256_hex(
            format!(
                "b16-rewrap|tenant={}|from={}|to={}|plaintext={after}|seq={}",
                self.tenant_id, claim.from_epoch, claim.to_epoch, self.admitted_ops
            )
            .as_bytes(),
        );
        Ok(CryptoOpReceipt {
            tenant_id: self.tenant_id.clone(),
            epoch: claim.to_epoch,
            plaintext_digest: after,
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

    #[test]
    fn b16_clean_rotation_rewrap_shred_and_active_write() {
        let mut gate = KeyRotationShreddingGate::new("tenant-alpha", hex64('1')).expect("init");
        let rot = gate.rotate_to_epoch(2, hex64('2')).expect("rotate 1->2");
        assert_eq!(rot.active_epoch, 2);

        let rewrap = gate
            .admit_rewrap(&CryptoRewrapClaim {
                tenant_id: "tenant-alpha".to_owned(),
                from_epoch: 1,
                to_epoch: 2,
                before_plaintext_digest: hex64('a'),
                after_plaintext_digest: hex64('a'),
            })
            .expect("rewrap preserves plaintext digest");
        assert_eq!(rewrap.epoch, 2);

        let tomb = gate
            .shred_retired_epoch(1, "gdpr-erasure-completed")
            .expect("shred epoch 1");
        assert_eq!(tomb.shredded_epoch, 1);
        assert!(gate.is_epoch_shredded(1));

        let write = gate
            .admit_write(&CryptoWriteClaim {
                tenant_id: "tenant-alpha".to_owned(),
                write_epoch: 2,
                plaintext_digest: hex64('b'),
            })
            .expect("write on active epoch 2");
        assert_eq!(write.epoch, 2);
        assert_eq!(gate.admitted_ops(), 2);
    }

    #[test]
    fn b16_refuses_stale_epoch_write_and_non_monotonic_rotation() {
        let mut gate = KeyRotationShreddingGate::new("tenant-alpha", hex64('1')).expect("init");
        assert!(matches!(
            gate.rotate_to_epoch(3, hex64('3')),
            Err(KeyRotationRejection::NonMonotonicEpochRotation { .. })
        ));
        gate.rotate_to_epoch(2, hex64('2')).expect("rotate 1->2");
        assert!(matches!(
            gate.admit_write(&CryptoWriteClaim {
                tenant_id: "tenant-alpha".to_owned(),
                write_epoch: 1,
                plaintext_digest: hex64('a'),
            }),
            Err(KeyRotationRejection::StaleWriteEpoch { .. })
        ));
    }

    #[test]
    fn b16_refuses_shredded_epoch_use_and_active_epoch_shred() {
        let mut gate = KeyRotationShreddingGate::new("tenant-alpha", hex64('1')).expect("init");
        assert!(matches!(
            gate.shred_retired_epoch(1, "premature"),
            Err(KeyRotationRejection::ActiveEpochCannotBeShreddedWithoutRotation { .. })
        ));
        gate.rotate_to_epoch(2, hex64('2')).expect("rotate");
        gate.shred_retired_epoch(1, "retired").expect("shred");
        assert!(matches!(
            gate.admit_rewrap(&CryptoRewrapClaim {
                tenant_id: "tenant-alpha".to_owned(),
                from_epoch: 1,
                to_epoch: 2,
                before_plaintext_digest: hex64('a'),
                after_plaintext_digest: hex64('a'),
            }),
            Err(KeyRotationRejection::EpochShredded { epoch: 1, .. })
        ));
    }

    #[test]
    fn b16_refuses_mutated_plaintext_rewrap_and_cross_tenant_aad() {
        let mut gate = KeyRotationShreddingGate::new("tenant-alpha", hex64('1')).expect("init");
        gate.rotate_to_epoch(2, hex64('2')).expect("rotate");
        assert!(matches!(
            gate.admit_rewrap(&CryptoRewrapClaim {
                tenant_id: "tenant-alpha".to_owned(),
                from_epoch: 1,
                to_epoch: 2,
                before_plaintext_digest: hex64('a'),
                after_plaintext_digest: hex64('f'),
            }),
            Err(KeyRotationRejection::PlaintextDigestMutatedDuringRewrap { .. })
        ));
        assert!(matches!(
            gate.admit_write(&CryptoWriteClaim {
                tenant_id: "tenant-beta".to_owned(),
                write_epoch: 2,
                plaintext_digest: hex64('a'),
            }),
            Err(KeyRotationRejection::TenantMismatch { .. })
        ));
    }
}
