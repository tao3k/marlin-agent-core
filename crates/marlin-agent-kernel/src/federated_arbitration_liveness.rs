// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! B13 Federated Arbitration Liveness & Bounded Escalation Gate.
//!
//! Enforces the formal invariants of `FederatedArbitrationLiveness.qnt` /
//! `MarlinBackendFederatedArbitrationOverlay.qnt` and `FederatedArbitrationLiveness.lean`:
//!   1. Strict retry budget (`attempts < max_attempts`): unbounded automatic retries are refused.
//!   2. Deterministic wait-for cycle / deadlock detection: any cycle in cross-actor wait edges
//!      latches deadlock, blocking further retries or clean admission until escalated/aborted.
//!   3. Absorbing terminal phases (`Admitted` vs `Escalated`): mutually exclusive and immutable;
//!      any terminal re-entry attempt fails closed.

use std::collections::{BTreeMap, BTreeSet};

use crate::governance_admission::governance_sha256_bytes;

fn sha256_hex(data: &[u8]) -> String {
    let digest = governance_sha256_bytes(data);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Absorbing lifecycle phase of a federated arbitration case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrationPhase {
    Active,
    Admitted,
    Escalated,
}

/// Per-case arbitration claim submitted for B13 bounded liveness admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationClaim {
    pub case_id: String,
    pub wait_for_edges: Vec<(String, String)>,
}

/// Machine-checkable fail-closed refusal reasons for B13 arbitration liveness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbitrationRejection {
    InvalidRetryBudget { max_attempts: u32 },
    RetryBudgetExhausted { case_id: String, attempts: u32, max_attempts: u32 },
    DeadlockCycleDetected { case_id: String, witness_node: String },
    TerminalReentryRefused { case_id: String, phase: ArbitrationPhase },
    EscalationNotJustified { case_id: String, attempts: u32, max_attempts: u32 },
}

/// Deterministic admission or escalation receipt for a B13 arbitration case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationReceipt {
    pub case_id: String,
    pub phase: ArbitrationPhase,
    pub attempts: u32,
    pub deadlock_detected: bool,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArbitrationCaseState {
    attempts: u32,
    deadlock_detected: bool,
    phase: ArbitrationPhase,
}

impl Default for ArbitrationCaseState {
    fn default() -> Self {
        Self {
            attempts: 0,
            deadlock_detected: false,
            phase: ArbitrationPhase::Active,
        }
    }
}

/// Stateful B13 federated arbitration liveness gate with bounded retry and cycle detection.
#[derive(Debug, Clone)]
pub struct FederatedArbitrationLivenessGate {
    max_attempts: u32,
    cases: BTreeMap<String, ArbitrationCaseState>,
}

impl FederatedArbitrationLivenessGate {
    pub fn new(max_attempts: u32) -> Result<Self, ArbitrationRejection> {
        if max_attempts == 0 {
            return Err(ArbitrationRejection::InvalidRetryBudget { max_attempts });
        }
        Ok(Self {
            max_attempts,
            cases: BTreeMap::new(),
        })
    }

    #[must_use]
    pub fn phase_of(&self, case_id: &str) -> ArbitrationPhase {
        self.cases
            .get(case_id)
            .map_or(ArbitrationPhase::Active, |c| c.phase)
    }

    #[must_use]
    pub fn attempts_of(&self, case_id: &str) -> u32 {
        self.cases.get(case_id).map_or(0, |c| c.attempts)
    }

    fn detect_cycle(edges: &[(String, String)]) -> Option<String> {
        let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut nodes: BTreeSet<&str> = BTreeSet::new();
        for (src, dst) in edges {
            adj.entry(src.as_str()).or_default().push(dst.as_str());
            nodes.insert(src.as_str());
            nodes.insert(dst.as_str());
        }
        // 0 = unvisited, 1 = visiting (on current stack), 2 = done
        let mut color: BTreeMap<&str, u8> = BTreeMap::new();
        fn dfs<'a>(
            node: &'a str,
            adj: &BTreeMap<&'a str, Vec<&'a str>>,
            color: &mut BTreeMap<&'a str, u8>,
        ) -> Option<String> {
            color.insert(node, 1);
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    match color.get(next).copied().unwrap_or(0) {
                        1 => return Some(next.to_owned()),
                        0 => {
                            if let Some(witness) = dfs(next, adj, color) {
                                return Some(witness);
                            }
                        }
                        _ => {}
                    }
                }
            }
            color.insert(node, 2);
            None
        }
        for node in nodes {
            if color.get(node).copied().unwrap_or(0) == 0 {
                if let Some(witness) = dfs(node, &adj, &mut color) {
                    return Some(witness);
                }
            }
        }
        None
    }

    /// Record a bounded automatic retry; fails closed on terminal phase, deadlock, or exhausted budget.
    pub fn attempt_auto_retry(
        &mut self,
        claim: &ArbitrationClaim,
    ) -> Result<u32, ArbitrationRejection> {
        let entry = self.cases.entry(claim.case_id.clone()).or_default();
        if entry.phase != ArbitrationPhase::Active {
            return Err(ArbitrationRejection::TerminalReentryRefused {
                case_id: claim.case_id.clone(),
                phase: entry.phase,
            });
        }
        if let Some(witness) = Self::detect_cycle(&claim.wait_for_edges) {
            entry.deadlock_detected = true;
            return Err(ArbitrationRejection::DeadlockCycleDetected {
                case_id: claim.case_id.clone(),
                witness_node: witness,
            });
        }
        if entry.deadlock_detected {
            return Err(ArbitrationRejection::DeadlockCycleDetected {
                case_id: claim.case_id.clone(),
                witness_node: "latched-deadlock".to_owned(),
            });
        }
        if entry.attempts >= self.max_attempts {
            return Err(ArbitrationRejection::RetryBudgetExhausted {
                case_id: claim.case_id.clone(),
                attempts: entry.attempts,
                max_attempts: self.max_attempts,
            });
        }
        entry.attempts += 1;
        Ok(entry.attempts)
    }

    /// Escalate/abort an active case once a deadlock cycle is detected or retry budget is exhausted.
    pub fn escalate_or_abort(
        &mut self,
        case_id: &str,
    ) -> Result<ArbitrationReceipt, ArbitrationRejection> {
        let entry = self.cases.entry(case_id.to_owned()).or_default();
        if entry.phase != ArbitrationPhase::Active {
            return Err(ArbitrationRejection::TerminalReentryRefused {
                case_id: case_id.to_owned(),
                phase: entry.phase,
            });
        }
        if !entry.deadlock_detected && entry.attempts < self.max_attempts {
            return Err(ArbitrationRejection::EscalationNotJustified {
                case_id: case_id.to_owned(),
                attempts: entry.attempts,
                max_attempts: self.max_attempts,
            });
        }
        entry.phase = ArbitrationPhase::Escalated;
        let receipt_digest = sha256_hex(
            format!(
                "b13|case={case_id}|phase=escalated|attempts={}|deadlock={}",
                entry.attempts, entry.deadlock_detected
            )
            .as_bytes(),
        );
        Ok(ArbitrationReceipt {
            case_id: case_id.to_owned(),
            phase: ArbitrationPhase::Escalated,
            attempts: entry.attempts,
            deadlock_detected: entry.deadlock_detected,
            receipt_digest,
        })
    }

    /// Admit an active, deadlock-free arbitration case within its bounded retry budget.
    pub fn admit_clean(
        &mut self,
        claim: &ArbitrationClaim,
    ) -> Result<ArbitrationReceipt, ArbitrationRejection> {
        let entry = self.cases.entry(claim.case_id.clone()).or_default();
        if entry.phase != ArbitrationPhase::Active {
            return Err(ArbitrationRejection::TerminalReentryRefused {
                case_id: claim.case_id.clone(),
                phase: entry.phase,
            });
        }
        if let Some(witness) = Self::detect_cycle(&claim.wait_for_edges) {
            entry.deadlock_detected = true;
            return Err(ArbitrationRejection::DeadlockCycleDetected {
                case_id: claim.case_id.clone(),
                witness_node: witness,
            });
        }
        if entry.deadlock_detected {
            return Err(ArbitrationRejection::DeadlockCycleDetected {
                case_id: claim.case_id.clone(),
                witness_node: "latched-deadlock".to_owned(),
            });
        }
        if entry.attempts >= self.max_attempts {
            return Err(ArbitrationRejection::RetryBudgetExhausted {
                case_id: claim.case_id.clone(),
                attempts: entry.attempts,
                max_attempts: self.max_attempts,
            });
        }
        entry.phase = ArbitrationPhase::Admitted;
        let receipt_digest = sha256_hex(
            format!(
                "b13|case={}|phase=admitted|attempts={}|deadlock=false",
                claim.case_id, entry.attempts
            )
            .as_bytes(),
        );
        Ok(ArbitrationReceipt {
            case_id: claim.case_id.clone(),
            phase: ArbitrationPhase::Admitted,
            attempts: entry.attempts,
            deadlock_detected: false,
            receipt_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b13_admits_acyclic_case_and_blocks_terminal_reentry() {
        let mut gate = FederatedArbitrationLivenessGate::new(2).expect("valid budget");
        let claim = ArbitrationClaim {
            case_id: "arb-01".to_owned(),
            wait_for_edges: vec![("agent-a".to_owned(), "agent-b".to_owned())],
        };
        assert_eq!(gate.attempt_auto_retry(&claim).expect("retry 1"), 1);
        let receipt = gate.admit_clean(&claim).expect("admit clean");
        assert_eq!(receipt.phase, ArbitrationPhase::Admitted);
        assert_eq!(receipt.receipt_digest.len(), 64);

        // Terminal phase is absorbing: retry, re-admit, and escalate all fail closed.
        assert!(matches!(
            gate.attempt_auto_retry(&claim),
            Err(ArbitrationRejection::TerminalReentryRefused {
                phase: ArbitrationPhase::Admitted,
                ..
            })
        ));
        assert!(matches!(
            gate.escalate_or_abort("arb-01"),
            Err(ArbitrationRejection::TerminalReentryRefused {
                phase: ArbitrationPhase::Admitted,
                ..
            })
        ));
    }

    #[test]
    fn b13_refuses_unbounded_retry_and_forces_escalation() {
        let mut gate = FederatedArbitrationLivenessGate::new(2).expect("valid budget");
        let claim = ArbitrationClaim {
            case_id: "arb-budget".to_owned(),
            wait_for_edges: Vec::new(),
        };
        assert_eq!(gate.attempt_auto_retry(&claim).expect("retry 1"), 1);
        assert_eq!(gate.attempt_auto_retry(&claim).expect("retry 2"), 2);
        assert!(matches!(
            gate.attempt_auto_retry(&claim),
            Err(ArbitrationRejection::RetryBudgetExhausted { .. })
        ));
        assert!(matches!(
            gate.admit_clean(&claim),
            Err(ArbitrationRejection::RetryBudgetExhausted { .. })
        ));
        let esc = gate.escalate_or_abort("arb-budget").expect("escalate");
        assert_eq!(esc.phase, ArbitrationPhase::Escalated);
        assert!(matches!(
            gate.attempt_auto_retry(&claim),
            Err(ArbitrationRejection::TerminalReentryRefused {
                phase: ArbitrationPhase::Escalated,
                ..
            })
        ));
    }

    #[test]
    fn b13_detects_wait_for_deadlock_cycle_and_blocks_retry_until_escalated() {
        let mut gate = FederatedArbitrationLivenessGate::new(3).expect("valid budget");
        let cyclic_claim = ArbitrationClaim {
            case_id: "arb-deadlock".to_owned(),
            wait_for_edges: vec![
                ("org-a".to_owned(), "org-b".to_owned()),
                ("org-b".to_owned(), "org-a".to_owned()),
            ],
        };
        assert!(matches!(
            gate.attempt_auto_retry(&cyclic_claim),
            Err(ArbitrationRejection::DeadlockCycleDetected { .. })
        ));
        // Even with edges cleared, latched deadlock forbids retry or clean admission.
        let cleared_claim = ArbitrationClaim {
            case_id: "arb-deadlock".to_owned(),
            wait_for_edges: Vec::new(),
        };
        assert!(matches!(
            gate.admit_clean(&cleared_claim),
            Err(ArbitrationRejection::DeadlockCycleDetected { .. })
        ));
        let esc = gate
            .escalate_or_abort("arb-deadlock")
            .expect("escalate deadlock");
        assert_eq!(esc.phase, ArbitrationPhase::Escalated);
        assert!(esc.deadlock_detected);
    }
}
