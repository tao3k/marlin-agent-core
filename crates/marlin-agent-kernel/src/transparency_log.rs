// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authoritative Rust B9 Verifiable Transparency Log (RFC 6962) & Split-View
//! Equivocation Defense for Marlin governance receipts.
//!
//! Refined by:
//! - `engineering.defense.transparency-log-equivocation` (`TransparencyLogEquivocation.qnt`)
//! - `DeterministicEngineering.Engineering.MerkleAsyncAndMemoization` (Lean 4 / LeanPoo)
//! - `dev/quint/threat/MarlinBackendTransparencyLogOverlay.qnt`
//!
//! Zero external dependencies: uses FIPS 180-4 SHA-256 (`governance_sha256_bytes`)
//! with strict RFC 6962 §2.1 domain separation (`0x00` leaf prefix, `0x01` internal
//! node prefix) to construct and verify:
//! - Merkle Inclusion Proofs (RFC 6962 §2.1.1),
//! - Append-Only Consistency Proofs (RFC 6962 §2.1.2),
//! - Self-authenticating Split-View Equivocation & Rollback Fraud Proofs.

use std::{error::Error, fmt, fmt::Write as _};

use crate::governance_admission::{GovernanceProofDigest, governance_sha256_bytes};

fn hash_leaf(leaf: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(1 + leaf.len());
    buf.push(0x00);
    buf.extend_from_slice(leaf);
    governance_sha256_bytes(&buf)
}

fn hash_children(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 65];
    buf[0] = 0x01;
    buf[1..33].copy_from_slice(left);
    buf[33..65].copy_from_slice(right);
    governance_sha256_bytes(&buf)
}

fn largest_power_of_two_less_than(n: usize) -> usize {
    debug_assert!(n >= 2);
    1usize << (usize::BITS - 1 - (n - 1).leading_zeros())
}

fn mth(leaves: &[Vec<u8>]) -> [u8; 32] {
    match leaves.len() {
        0 => governance_sha256_bytes(&[]),
        1 => hash_leaf(&leaves[0]),
        n => {
            let k = largest_power_of_two_less_than(n);
            let left = mth(&leaves[..k]);
            let right = mth(&leaves[k..]);
            hash_children(&left, &right)
        }
    }
}

fn audit_path(m: usize, leaves: &[Vec<u8>]) -> Vec<[u8; 32]> {
    let n = leaves.len();
    if n <= 1 {
        return Vec::new();
    }
    let k = largest_power_of_two_less_than(n);
    if m < k {
        let mut p = audit_path(m, &leaves[..k]);
        p.push(mth(&leaves[k..]));
        p
    } else {
        let mut p = audit_path(m - k, &leaves[k..]);
        p.push(mth(&leaves[..k]));
        p
    }
}

fn consistency_subproof(m: usize, leaves: &[Vec<u8>], complete_subtree: bool) -> Vec<[u8; 32]> {
    let n = leaves.len();
    if m == n {
        return if complete_subtree {
            Vec::new()
        } else {
            vec![mth(leaves)]
        };
    }
    let k = largest_power_of_two_less_than(n);
    if m <= k {
        let mut p = consistency_subproof(m, &leaves[..k], complete_subtree);
        p.push(mth(&leaves[k..]));
        p
    } else {
        let mut p = consistency_subproof(m - k, &leaves[k..], false);
        p.push(mth(&leaves[..k]));
        p
    }
}

fn to_proof_digest(bytes: &[u8; 32]) -> GovernanceProofDigest {
    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in bytes {
        let _ = write!(&mut hex, "{b:02x}");
    }
    GovernanceProofDigest::new(hex).expect("32-byte SHA-256 hex always valid")
}

/// Immutable RFC 6962 Tree Head binding `tree_size` to its Merkle `root_hash`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceTreeHead {
    pub tree_size: usize,
    pub root_hash: [u8; 32],
}

impl GovernanceTreeHead {
    pub fn root_digest(&self) -> GovernanceProofDigest {
        to_proof_digest(&self.root_hash)
    }
}

/// RFC 6962 §2.1.1 Merkle inclusion proof for one logged governance entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleInclusionProof {
    pub leaf_index: usize,
    pub tree_size: usize,
    pub audit_path: Vec<[u8; 32]>,
}

/// RFC 6962 §2.1.2 Append-only consistency proof between two tree sizes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleConsistencyProof {
    pub first_size: usize,
    pub second_size: usize,
    pub consistency_path: Vec<[u8; 32]>,
}

/// Self-authenticating fraud proof emitted on split-view equivocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitViewEquivocationFraudProof {
    pub tree_size: usize,
    pub first_root: GovernanceProofDigest,
    pub conflicting_root: GovernanceProofDigest,
}

/// Typed fail-closed verification errors for RFC 6962 transparency log proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransparencyLogRejection {
    LeafIndexOutOfBounds { leaf_index: usize, tree_size: usize },
    InvalidInclusionProof,
    HistoryRollback { prev_size: usize, next_size: usize },
    SplitViewEquivocation(SplitViewEquivocationFraudProof),
    ConsistencyProofMismatch,
}

impl fmt::Display for TransparencyLogRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LeafIndexOutOfBounds {
                leaf_index,
                tree_size,
            } => write!(f, "leaf index {leaf_index} out of bounds for tree size {tree_size}"),
            Self::InvalidInclusionProof => write!(f, "RFC 6962 Merkle inclusion proof failed"),
            Self::HistoryRollback {
                prev_size,
                next_size,
            } => write!(
                f,
                "transparency log rollback refused: tree size shrank from {prev_size} to {next_size}"
            ),
            Self::SplitViewEquivocation(fp) => write!(
                f,
                "split-view equivocation at tree_size={}: {} != {}",
                fp.tree_size,
                fp.first_root.as_str(),
                fp.conflicting_root.as_str()
            ),
            Self::ConsistencyProofMismatch => {
                write!(f, "RFC 6962 append-only consistency proof failed (history rewrite)")
            }
        }
    }
}

impl Error for TransparencyLogRejection {}

/// Append-only RFC 6962 Merkle Transparency Log for Marlin governance receipts.
#[derive(Clone, Debug, Default)]
pub struct GovernanceTransparencyLog {
    leaves: Vec<Vec<u8>>,
}

impl GovernanceTransparencyLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    pub fn append(&mut self, entry: impl Into<Vec<u8>>) -> usize {
        let idx = self.leaves.len();
        self.leaves.push(entry.into());
        idx
    }

    pub fn tree_head(&self) -> GovernanceTreeHead {
        GovernanceTreeHead {
            tree_size: self.leaves.len(),
            root_hash: mth(&self.leaves),
        }
    }

    pub fn inclusion_proof(
        &self,
        leaf_index: usize,
    ) -> Result<MerkleInclusionProof, TransparencyLogRejection> {
        if leaf_index >= self.leaves.len() {
            return Err(TransparencyLogRejection::LeafIndexOutOfBounds {
                leaf_index,
                tree_size: self.leaves.len(),
            });
        }
        Ok(MerkleInclusionProof {
            leaf_index,
            tree_size: self.leaves.len(),
            audit_path: audit_path(leaf_index, &self.leaves),
        })
    }

    pub fn consistency_proof(
        &self,
        first_size: usize,
    ) -> Result<MerkleConsistencyProof, TransparencyLogRejection> {
        let second_size = self.leaves.len();
        if first_size > second_size {
            return Err(TransparencyLogRejection::HistoryRollback {
                prev_size: first_size,
                next_size: second_size,
            });
        }
        let path = if first_size == 0 || first_size == second_size {
            Vec::new()
        } else {
            consistency_subproof(first_size, &self.leaves, true)
        };
        Ok(MerkleConsistencyProof {
            first_size,
            second_size,
            consistency_path: path,
        })
    }
}

/// Verifies an RFC 6962 §2.1.1 Merkle inclusion proof for `leaf_data` against `head`.
pub fn verify_inclusion_proof(
    leaf_data: &[u8],
    proof: &MerkleInclusionProof,
    head: &GovernanceTreeHead,
) -> Result<(), TransparencyLogRejection> {
    if proof.tree_size != head.tree_size || proof.leaf_index >= proof.tree_size {
        return Err(TransparencyLogRejection::LeafIndexOutOfBounds {
            leaf_index: proof.leaf_index,
            tree_size: head.tree_size,
        });
    }
    let mut fn_idx = proof.leaf_index;
    let mut sn = proof.tree_size - 1;
    let mut r = hash_leaf(leaf_data);

    for p in &proof.audit_path {
        if sn == 0 {
            return Err(TransparencyLogRejection::InvalidInclusionProof);
        }
        if (fn_idx & 1) == 1 || fn_idx == sn {
            r = hash_children(p, &r);
            while (fn_idx & 1) == 0 && fn_idx != 0 {
                fn_idx >>= 1;
                sn >>= 1;
            }
        } else {
            r = hash_children(&r, p);
        }
        fn_idx >>= 1;
        sn >>= 1;
    }

    if sn != 0 || fn_idx != 0 || r != head.root_hash {
        return Err(TransparencyLogRejection::InvalidInclusionProof);
    }
    Ok(())
}

/// Verifies append-only progression between `prev` and `next` tree heads, detecting
/// history rollback, split-view equivocation, and non-prefix history rewrites.
pub fn verify_tree_head_progression(
    prev: &GovernanceTreeHead,
    next: &GovernanceTreeHead,
    proof: &MerkleConsistencyProof,
) -> Result<(), TransparencyLogRejection> {
    if next.tree_size < prev.tree_size {
        return Err(TransparencyLogRejection::HistoryRollback {
            prev_size: prev.tree_size,
            next_size: next.tree_size,
        });
    }
    if next.tree_size == prev.tree_size {
        if prev.root_hash != next.root_hash {
            return Err(TransparencyLogRejection::SplitViewEquivocation(
                SplitViewEquivocationFraudProof {
                    tree_size: prev.tree_size,
                    first_root: prev.root_digest(),
                    conflicting_root: next.root_digest(),
                },
            ));
        }
        if (prev.tree_size == 0 && prev.root_hash != governance_sha256_bytes(&[]))
            || proof.first_size != prev.tree_size
            || proof.second_size != next.tree_size
            || !proof.consistency_path.is_empty()
        {
            return Err(TransparencyLogRejection::ConsistencyProofMismatch);
        }
        return Ok(());
    }
    if prev.tree_size == 0 {
        if prev.root_hash != governance_sha256_bytes(&[])
            || proof.first_size != 0
            || proof.second_size != next.tree_size
            || !proof.consistency_path.is_empty()
        {
            return Err(TransparencyLogRejection::ConsistencyProofMismatch);
        }
        return Ok(());
    }
    if proof.first_size != prev.tree_size
        || proof.second_size != next.tree_size
        || proof.consistency_path.is_empty()
    {
        return Err(TransparencyLogRejection::ConsistencyProofMismatch);
    }

    let mut fn_idx = prev.tree_size - 1;
    let mut sn = next.tree_size - 1;
    while (fn_idx & 1) == 1 {
        fn_idx >>= 1;
        sn >>= 1;
    }

    let mut path_iter = proof.consistency_path.iter();
    let (mut fr, mut sr) = if prev.tree_size.is_power_of_two() {
        (prev.root_hash, prev.root_hash)
    } else {
        let Some(&first_node) = path_iter.next() else {
            return Err(TransparencyLogRejection::ConsistencyProofMismatch);
        };
        (first_node, first_node)
    };

    for &c in path_iter {
        if sn == 0 {
            return Err(TransparencyLogRejection::ConsistencyProofMismatch);
        }
        if (fn_idx & 1) == 1 || fn_idx == sn {
            fr = hash_children(&c, &fr);
            sr = hash_children(&c, &sr);
            while (fn_idx & 1) == 0 && fn_idx != 0 {
                fn_idx >>= 1;
                sn >>= 1;
            }
        } else {
            sr = hash_children(&sr, &c);
        }
        fn_idx >>= 1;
        sn >>= 1;
    }

    if sn != 0 || fr != prev.root_hash || sr != next.root_hash {
        return Err(TransparencyLogRejection::ConsistencyProofMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_rfc6962_inclusion_and_consistency_proofs_across_all_tree_sizes() {
        let mut log = GovernanceTransparencyLog::new();
        let mut heads = vec![log.tree_head()];

        for i in 0..12usize {
            let entry = format!("governance-receipt-seq-{i}").into_bytes();
            let idx = log.append(entry);
            let head = log.tree_head();
            heads.push(head.clone());

            // Verify inclusion proof for every historical leaf up to current size.
            for j in 0..=idx {
                let proof = log.inclusion_proof(j).expect("inclusion proof");
                let leaf = format!("governance-receipt-seq-{j}").into_bytes();
                verify_inclusion_proof(&leaf, &proof, &head)
                    .expect("valid inclusion proof must verify");
            }

            // Verify append-only consistency proof from every earlier tree head.
            for prev in &heads {
                let cproof = log
                    .consistency_proof(prev.tree_size)
                    .expect("consistency proof");
                verify_tree_head_progression(prev, &head, &cproof)
                    .expect("valid consistency proof must verify");
            }
        }
    }

    #[test]
    fn detects_split_view_equivocation_rollback_and_history_rewrite_fail_closed() {
        let mut log_a = GovernanceTransparencyLog::new();
        let mut log_b = GovernanceTransparencyLog::new();
        for i in 0..4 {
            let entry = format!("receipt-{i}").into_bytes();
            log_a.append(entry.clone());
            log_b.append(entry);
        }
        let head_4 = log_a.tree_head();

        // 1. Tampered leaf breaks inclusion proof.
        let p2 = log_a.inclusion_proof(2).unwrap();
        assert_eq!(
            verify_inclusion_proof(b"receipt-2-tampered", &p2, &head_4),
            Err(TransparencyLogRejection::InvalidInclusionProof)
        );

        // 2. Split-view equivocation at tree_size=5 emits fraud proof.
        log_a.append(b"receipt-4-world-A".to_vec());
        log_b.append(b"receipt-4-world-B".to_vec());
        let head_5a = log_a.tree_head();
        let head_5b = log_b.tree_head();
        let empty_proof = MerkleConsistencyProof {
            first_size: 5,
            second_size: 5,
            consistency_path: Vec::new(),
        };
        assert!(matches!(
            verify_tree_head_progression(&head_5a, &head_5b, &empty_proof),
            Err(TransparencyLogRejection::SplitViewEquivocation(fp)) if fp.tree_size == 5
        ));

        // 3. History rollback (5 -> 4) refused fail-closed.
        assert_eq!(
            verify_tree_head_progression(&head_5a, &head_4, &empty_proof),
            Err(TransparencyLogRejection::HistoryRollback {
                prev_size: 5,
                next_size: 4,
            })
        );

        // 4. Rewritten history fails consistency proof against earlier head_4.
        let mut rewritten = GovernanceTransparencyLog::new();
        rewritten.append(b"receipt-0".to_vec());
        rewritten.append(b"receipt-1-REWRITTEN".to_vec());
        rewritten.append(b"receipt-2".to_vec());
        rewritten.append(b"receipt-3".to_vec());
        rewritten.append(b"receipt-4-world-A".to_vec());
        let bad_head_5 = rewritten.tree_head();
        let bad_proof = rewritten.consistency_proof(4).unwrap();
        assert_eq!(
            verify_tree_head_progression(&head_4, &bad_head_5, &bad_proof),
            Err(TransparencyLogRejection::ConsistencyProofMismatch)
        );
    }
}
