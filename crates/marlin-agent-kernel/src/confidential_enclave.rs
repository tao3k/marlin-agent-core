// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B15 Confidential Compute / Hardware TEE Attestation & Sealed Key Release Gate.
//!
//! Enforces the formal invariants of `ConfidentialEnclaveAttestation.qnt` /
//! `MarlinBackendConfidentialEnclaveOverlay.qnt` and `ConfidentialEnclaveAttestation.lean`:
//!   1. Debug-enabled enclaves are unconditionally refused (anti-debug key leak).
//!   2. Hardware quote signature must verify and `measurement_digest` must match an
//!      allowlisted 64-hex SHA-256 enclave measurement (pinned to B14 artifact if bound).
//!   3. Hardware TCB security version number (`tcb_svn`) must meet or exceed the monotonic floor.
//!   4. Each issued challenge nonce may be redeemed at most once (blocks quote replay attacks).

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

/// Hardware TEE attestation quote submitted to unlock sealed runtime credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclaveAttestationQuote {
    pub enclave_id: String,
    pub debug_mode_enabled: bool,
    pub measurement_digest: String,
    pub tcb_svn: u32,
    pub challenge_nonce: String,
    pub bound_b14_artifact_digest: Option<String>,
    pub quote_signature_verified: bool,
}

/// Machine-checkable failure reasons for B15 enclave attestation & key release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfidentialEnclaveRejection {
    DebugEnclaveRefused {
        enclave_id: String,
    },
    InvalidQuoteSignature {
        enclave_id: String,
    },
    InvalidMeasurementDigest {
        digest: String,
    },
    UnmeasuredOrUnpinnedEnclave {
        measurement_digest: String,
    },
    StaleTcbSvn {
        actual_svn: u32,
        min_required_svn: u32,
    },
    InvalidOrReplayedNonce {
        nonce: String,
    },
    B14ArtifactBindingMismatch {
        expected: String,
        actual: Option<String>,
    },
}

/// Deterministic key-release receipt bound to measurement, TCB SVN, and consumed nonce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclaveKeyReleaseReceipt {
    pub enclave_id: String,
    pub measurement_digest: String,
    pub tcb_svn: u32,
    pub receipt_digest: String,
}

/// Stateful B15 hardware TEE attestation gate with single-use challenge nonces and TCB floor.
#[derive(Debug, Clone)]
pub struct ConfidentialEnclaveGate {
    min_tcb_svn: u32,
    /// `measurement_digest -> optional required B14 artifact digest`.
    allowlisted_measurements: BTreeMap<String, Option<String>>,
    issued_nonces: BTreeSet<String>,
    consumed_nonces: BTreeSet<String>,
    keys_released: u64,
}

impl ConfidentialEnclaveGate {
    #[must_use]
    pub fn new(min_tcb_svn: u32) -> Self {
        Self {
            min_tcb_svn,
            allowlisted_measurements: BTreeMap::new(),
            issued_nonces: BTreeSet::new(),
            consumed_nonces: BTreeSet::new(),
            keys_released: 0,
        }
    }

    /// Monotonically raise the minimum hardware TCB SVN floor.
    pub fn raise_min_tcb_svn(&mut self, new_min_svn: u32) {
        if new_min_svn > self.min_tcb_svn {
            self.min_tcb_svn = new_min_svn;
        }
    }

    /// Register a trusted enclave measurement (optionally bound to a B14 artifact digest).
    pub fn register_measurement(
        &mut self,
        measurement_digest: impl Into<String>,
        bound_b14_artifact_digest: Option<String>,
    ) -> Result<(), ConfidentialEnclaveRejection> {
        let meas = measurement_digest.into().to_ascii_lowercase();
        if !is_valid_sha256_hex(&meas) {
            return Err(ConfidentialEnclaveRejection::InvalidMeasurementDigest { digest: meas });
        }
        let bound = match bound_b14_artifact_digest {
            Some(art) => {
                let norm = art.to_ascii_lowercase();
                if !is_valid_sha256_hex(&norm) {
                    return Err(ConfidentialEnclaveRejection::InvalidMeasurementDigest {
                        digest: norm,
                    });
                }
                Some(norm)
            }
            None => None,
        };
        self.allowlisted_measurements.insert(meas, bound);
        Ok(())
    }

    /// Register a fresh challenge nonce eligible for one-time quote verification.
    pub fn issue_challenge_nonce(&mut self, nonce: impl Into<String>) {
        let n = nonce.into();
        if !n.trim().is_empty() && !self.consumed_nonces.contains(&n) {
            self.issued_nonces.insert(n);
        }
    }

    #[must_use]
    pub fn keys_released(&self) -> u64 {
        self.keys_released
    }

    /// Verify a TEE attestation quote fail-closed and consume its challenge nonce atomically.
    pub fn release_sealed_key(
        &mut self,
        quote: &EnclaveAttestationQuote,
    ) -> Result<EnclaveKeyReleaseReceipt, ConfidentialEnclaveRejection> {
        if quote.debug_mode_enabled {
            return Err(ConfidentialEnclaveRejection::DebugEnclaveRefused {
                enclave_id: quote.enclave_id.clone(),
            });
        }
        if !quote.quote_signature_verified {
            return Err(ConfidentialEnclaveRejection::InvalidQuoteSignature {
                enclave_id: quote.enclave_id.clone(),
            });
        }
        let meas = quote.measurement_digest.to_ascii_lowercase();
        if !is_valid_sha256_hex(&meas) {
            return Err(ConfidentialEnclaveRejection::InvalidMeasurementDigest {
                digest: quote.measurement_digest.clone(),
            });
        }
        let Some(expected_b14) = self.allowlisted_measurements.get(&meas) else {
            return Err(ConfidentialEnclaveRejection::UnmeasuredOrUnpinnedEnclave {
                measurement_digest: meas,
            });
        };
        if let Some(expected_art) = expected_b14 {
            let actual_art = quote
                .bound_b14_artifact_digest
                .as_ref()
                .map(|s| s.to_ascii_lowercase());
            if actual_art.as_deref() != Some(expected_art.as_str()) {
                return Err(ConfidentialEnclaveRejection::B14ArtifactBindingMismatch {
                    expected: expected_art.clone(),
                    actual: actual_art,
                });
            }
        }
        if quote.tcb_svn < self.min_tcb_svn {
            return Err(ConfidentialEnclaveRejection::StaleTcbSvn {
                actual_svn: quote.tcb_svn,
                min_required_svn: self.min_tcb_svn,
            });
        }
        if !self.issued_nonces.remove(&quote.challenge_nonce) {
            return Err(ConfidentialEnclaveRejection::InvalidOrReplayedNonce {
                nonce: quote.challenge_nonce.clone(),
            });
        }
        self.consumed_nonces.insert(quote.challenge_nonce.clone());
        self.keys_released = self.keys_released.saturating_add(1);
        let receipt_digest = sha256_hex(
            format!(
                "b15|enclave={}|meas={meas}|svn={}|nonce={}",
                quote.enclave_id, quote.tcb_svn, quote.challenge_nonce
            )
            .as_bytes(),
        );
        Ok(EnclaveKeyReleaseReceipt {
            enclave_id: quote.enclave_id.clone(),
            measurement_digest: meas,
            tcb_svn: quote.tcb_svn,
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

    fn setup_gate_and_quote() -> (ConfidentialEnclaveGate, EnclaveAttestationQuote) {
        let mut gate = ConfidentialEnclaveGate::new(14);
        gate.register_measurement(hex64('e'), Some(hex64('f')))
            .expect("register measurement");
        gate.issue_challenge_nonce("nonce-tee-001");
        let quote = EnclaveAttestationQuote {
            enclave_id: "tdx-enclave-eu-01".to_owned(),
            debug_mode_enabled: false,
            measurement_digest: hex64('e'),
            tcb_svn: 15,
            challenge_nonce: "nonce-tee-001".to_owned(),
            bound_b14_artifact_digest: Some(hex64('f')),
            quote_signature_verified: true,
        };
        (gate, quote)
    }

    #[test]
    fn b15_releases_key_for_clean_quote_and_blocks_nonce_replay() {
        let (mut gate, quote) = setup_gate_and_quote();
        let receipt = gate.release_sealed_key(&quote).expect("clean release");
        assert_eq!(receipt.receipt_digest.len(), 64);
        assert_eq!(gate.keys_released(), 1);
        assert!(matches!(
            gate.release_sealed_key(&quote),
            Err(ConfidentialEnclaveRejection::InvalidOrReplayedNonce { .. })
        ));
    }

    #[test]
    fn b15_refuses_debug_unmeasured_stale_tcb_and_b14_mismatch() {
        let (mut gate, quote) = setup_gate_and_quote();

        let mut debug_q = quote.clone();
        debug_q.debug_mode_enabled = true;
        assert!(matches!(
            gate.release_sealed_key(&debug_q),
            Err(ConfidentialEnclaveRejection::DebugEnclaveRefused { .. })
        ));

        let mut unmeasured_q = quote.clone();
        unmeasured_q.measurement_digest = hex64('9');
        assert!(matches!(
            gate.release_sealed_key(&unmeasured_q),
            Err(ConfidentialEnclaveRejection::UnmeasuredOrUnpinnedEnclave { .. })
        ));

        let mut stale_tcb_q = quote.clone();
        stale_tcb_q.tcb_svn = 10;
        assert!(matches!(
            gate.release_sealed_key(&stale_tcb_q),
            Err(ConfidentialEnclaveRejection::StaleTcbSvn { .. })
        ));

        let mut bad_b14_q = quote;
        bad_b14_q.bound_b14_artifact_digest = Some(hex64('0'));
        assert!(matches!(
            gate.release_sealed_key(&bad_b14_q),
            Err(ConfidentialEnclaveRejection::B14ArtifactBindingMismatch { .. })
        ));
    }
}
