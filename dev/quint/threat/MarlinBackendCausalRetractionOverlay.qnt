// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/execution/agent-tool-circuit-breaker/AgentToolCircuitBreaker"
// @countermodel causal-retraction-blocks-effect step=stepCommitAfterPremiseRetraction max-steps=6 itf="h.retractedEffectCommit == true"
// @countermodel monotonic-proof-epoch-fence step=stepAcceptStaleProofEnvelope max-steps=6 itf="h.staleProofAccepted == true"

// Marlin Rust+Scheme Higher-Order Governance overlay composing AgentToolCircuitBreaker
// with Bi-Temporal Truth Maintenance premise retraction and monotonic Scheme->Rust
// proof-envelope revocation epoch fencing.
module marlin_backend_causal_retraction_overlay {
  type State = {
    premiseRetracted: bool,
    proofEpochFloor: int,
    retractedEffectCommit: bool,
    staleProofAccepted: bool
  }

  action init = h' = {
    premiseRetracted: false,
    proofEpochFloor: 1,
    retractedEffectCommit: false,
    staleProofAccepted: false
  }

  // Bi-Temporal Truth Maintenance retracts a supporting premise -> fences & auto-trips breaker
  action retractCausalPremise =
    if (not(h.premiseRetracted) and h.proofEpochFloor < 4)
      h' = h
        .with("premiseRetracted", true)
        .with("breakerTripped", true)
        .with("proofEpochFloor", h.proofEpochFloor + 1)
    else
      h' = h

  // Restore clean causal support ONLY at a fresh proof epoch after human re-arm
  action reinstateCleanProofAt(suppliedEpoch: int): bool =
    if (h.premiseRetracted and not(h.breakerTripped) and suppliedEpoch >= h.proofEpochFloor)
      h' = h
        .with("premiseRetracted", false)
        .with("proofEpochFloor", suppliedEpoch)
        .with("staleProofAccepted", h.staleProofAccepted or suppliedEpoch < h.proofEpochFloor)
    else
      h' = h

  action reinstateCleanProof = {
    nondet suppliedEpoch = Set(1, 2, 3, 4).oneOf()
    reinstateCleanProofAt(suppliedEpoch)
  }

  action commitDerivedSideEffect =
    if (h.leaseActive and not(h.breakerTripped) and not(h.premiseRetracted) and epochCurrent(h.tripEpoch, h.leaseEpoch) and h.budgetUsed < 1)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("budgetUsed", h.budgetUsed + 1)
        .with("execWhileTripped", h.execWhileTripped or h.breakerTripped)
        .with("staleLeaseCommit", h.staleLeaseCommit or h.tripEpoch != h.leaseEpoch)
        .with("budgetOverrun", h.budgetOverrun or h.budgetUsed >= 1)
        .with("retractedEffectCommit", h.retractedEffectCommit or h.premiseRetracted)
    else
      h' = h

  action step = any {
    retractCausalPremise,
    reinstateCleanProof,
    commitDerivedSideEffect
  }

  val safety = not(h.retractedEffectCommit) and not(h.staleProofAccepted)

  // Countermodel 1: Side effect commits even though supporting causal premise was retracted
  action commitAfterPremiseRetractionUnchecked =
    if (h.leaseActive and h.premiseRetracted and h.budgetUsed < 1)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("budgetUsed", h.budgetUsed + 1)
        .with("retractedEffectCommit", true)
    else
      h' = h

  action stepCommitAfterPremiseRetraction = any { step, commitAfterPremiseRetractionUnchecked }

  // Countermodel 2: Stale pre-retraction proof envelope (suppliedEpoch < proofEpochFloor) accepted
  action acceptStaleProofUnchecked(suppliedEpoch: int): bool =
    if (h.premiseRetracted and suppliedEpoch < h.proofEpochFloor)
      h' = h
        .with("premiseRetracted", false)
        .with("staleProofAccepted", true)
    else
      h' = h

  action stepAcceptStaleProofEnvelope = any {
    step,
    {
      nondet suppliedEpoch = Set(1, 2, 3).oneOf()
      acceptStaleProofUnchecked(suppliedEpoch)
    }
  }

  run causalRetractionAndFreshProofRecoveryTest =
    init
      .then(acquireToolLease)
      .then(retractCausalPremise)
      .then(commitDerivedSideEffect)
      .then(rearmBreakerAs(false, true))
      .then(reinstateCleanProofAt(1))
      .then(reinstateCleanProofAt(2))
      .then(cancelStaleLease)
      .then(acquireToolLease)
      .then(commitDerivedSideEffect)
      .then(all {
        assert(h.budgetUsed == 1),
        assert(not(h.premiseRetracted)),
        assert(not(h.retractedEffectCommit)),
        assert(not(h.staleProofAccepted)),
        assert(safety),
        h' = h
      })

  run commitAfterPremiseRetractionCountermodelTest =
    init.then(acquireToolLease).then(retractCausalPremise).then(commitAfterPremiseRetractionUnchecked).then(checkUnsafe)

  run acceptStaleProofCountermodelTest =
    init.then(retractCausalPremise).then(acceptStaleProofUnchecked(1)).then(checkUnsafe)
}
