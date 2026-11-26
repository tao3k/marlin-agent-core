// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/execution/agent-tool-circuit-breaker/AgentToolCircuitBreaker"
// @countermodel restart-preserves-estop step=stepRestartClearsEStop max-steps=6 itf="h.restartClearedTrip == true"
// @countermodel restart-monotonic-epochs step=stepRestartRegressesEpoch max-steps=8 itf="h.restartEpochRegressed == true"
// @countermodel corrupt-wal-fails-closed step=stepCorruptWalFailsOpen max-steps=6 itf="h.corruptWalOpened == true"
// @countermodel corrupt-fact-graph-fails-closed step=stepCorruptFactGraphFailsOpen max-steps=6 itf="h.corruptFactGraphOpened == true"
// @countermodel cedar-forbid-overrides-permit step=stepCedarForbidBypassedByPermit max-steps=6 itf="h.cedarForbidBypassed == true"
// @countermodel retracted-derivation-denied step=stepRetractedDerivationCommits max-steps=6 itf="h.retractedDerivationCommitted == true"
// @countermodel scope-unless-blocks-sibling-exfil step=stepScopeUnlessBypassSiblingExfil max-steps=6 itf="h.scopeUnlessBypassed == true"
// @countermodel missing-context-key-fails-closed step=stepMissingContextKeyBypassesCondition max-steps=6 itf="h.missingContextBypassed == true"

// Marlin Durable Governance overlay composing AgentToolCircuitBreaker
// with Turso WAL / CAS MGV1+MGF1 checkpointing, OS process crash-restart recovery,
// tamper-evident SHA-256 checkpoint integrity verification, Cedar forbid-overrides-permit,
// and Bi-Temporal Truth Maintenance dynamic derivation invalidation.
module marlin_backend_durable_governance_overlay {
  type State = {
    walCommitted: bool,
    walTripped: bool,
    walTripEpoch: int,
    walProofEpoch: int,
    walFenced: bool,
    walCorrupted: bool,
    factGraphCorrupted: bool,
    liveProofEpoch: int,
    liveFenced: bool,
    dynamicDerivationRetracted: bool,
    restartClearedTrip: bool,
    restartEpochRegressed: bool,
    corruptWalOpened: bool,
    corruptFactGraphOpened: bool,
    cedarForbidBypassed: bool,
    retractedDerivationCommitted: bool,
    scopeUnlessBypassed: bool,
    missingContextBypassed: bool
  }

  action init = h' = {
    walCommitted: false,
    walTripped: false,
    walTripEpoch: 1,
    walProofEpoch: 1,
    walFenced: false,
    walCorrupted: false,
    factGraphCorrupted: false,
    liveProofEpoch: 1,
    liveFenced: false,
    dynamicDerivationRetracted: false,
    restartClearedTrip: false,
    restartEpochRegressed: false,
    corruptWalOpened: false,
    corruptFactGraphOpened: false,
    cedarForbidBypassed: false,
    retractedDerivationCommitted: false,
    scopeUnlessBypassed: false,
    missingContextBypassed: false
  }

  // Persist current live breaker + Scheme governance state into Turso WAL/CAS.
  action syncWalCheckpoint =
    if (not(h.walCorrupted) and not(h.factGraphCorrupted))
      h' = h
        .with("walCommitted", true)
        .with("walTripped", h.breakerTripped)
        .with("walTripEpoch", h.tripEpoch)
        .with("walProofEpoch", h.liveProofEpoch)
        .with("walFenced", h.liveFenced)
    else
      h' = h

  // Scheme ASCENT/TM incident trips breaker, fences actor, advances proof epoch, and flushes WAL.
  action applyAscentIncidentAndPersist =
    if (not(h.walCorrupted) and not(h.factGraphCorrupted) and h.liveProofEpoch < 4)
      h' = h
        .with("breakerTripped", true)
        .with("liveFenced", true)
        .with("dynamicDerivationRetracted", true)
        .with("liveProofEpoch", h.liveProofEpoch + 1)
        .with("walCommitted", true)
        .with("walTripped", true)
        .with("walTripEpoch", h.tripEpoch)
        .with("walProofEpoch", h.liveProofEpoch + 1)
        .with("walFenced", true)
    else
      h' = h

  // Adversary corrupts or bit-flips the persisted MGV1 Turso WAL blob on disk.
  action tamperWalOnDisk =
    if (h.walCommitted and not(h.walCorrupted))
      h' = h.with("walCorrupted", true)
    else
      h' = h

  // Adversary corrupts or bit-flips the companion MGF1 runtime fact-graph blob on disk.
  action tamperFactGraphOnDisk =
    if (h.walCommitted and not(h.factGraphCorrupted))
      h' = h.with("factGraphCorrupted", true)
    else
      h' = h

  // Safe crash + OS process restart recovery:
  // 1. In-flight volatile leases are wiped on crash.
  // 2. Corrupted/tampered MGV1 OR MGF1 WAL ALWAYS fails closed (breakerTripped=true, liveFenced=true).
  // 3. Valid WAL restores persisted tripped latch, monotonic epochs, and ASCENT fences.
  action restartAndRecoverDurable =
    if (h.walCorrupted or h.factGraphCorrupted)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("breakerTripped", true)
        .with("tripEpoch", h.walTripEpoch)
        .with("liveProofEpoch", h.walProofEpoch)
        .with("liveFenced", true)
        .with("corruptWalOpened", h.corruptWalOpened or false)
        .with("corruptFactGraphOpened", h.corruptFactGraphOpened or false)
    else if (h.walCommitted)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("breakerTripped", h.walTripped)
        .with("tripEpoch", h.walTripEpoch)
        .with("liveProofEpoch", h.walProofEpoch)
        .with("liveFenced", h.walFenced)
        .with("restartClearedTrip", h.restartClearedTrip or (h.walTripped and not(h.walTripped)))
        .with("restartEpochRegressed", h.restartEpochRegressed or (h.walTripEpoch < h.walTripEpoch))
    else
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)

  action step = any {
    syncWalCheckpoint,
    applyAscentIncidentAndPersist,
    tamperWalOnDisk,
    tamperFactGraphOnDisk,
    restartAndRecoverDurable
  }

  val safety =
    not(h.restartClearedTrip) and
    not(h.restartEpochRegressed) and
    not(h.corruptWalOpened) and
    not(h.corruptFactGraphOpened) and
    not(h.cedarForbidBypassed) and
    not(h.retractedDerivationCommitted) and
    not(h.scopeUnlessBypassed) and
    not(h.missingContextBypassed)

  // Countermodel 1: Restart-Reset Bypass — daemon reboots and wipes a persisted E-Stop latch.
  action restartClearsEStopUnchecked =
    if (h.walCommitted and h.walTripped and not(h.walCorrupted) and not(h.factGraphCorrupted))
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("breakerTripped", false)
        .with("restartClearedTrip", true)
    else
      h' = h

  action stepRestartClearsEStop = any { step, restartClearsEStopUnchecked }

  // Countermodel 2: Epoch Amnesia — daemon reboots back to epoch 1, enabling stale proof/lease replay.
  action restartRegressesEpochUnchecked =
    if (h.walCommitted and not(h.walCorrupted) and not(h.factGraphCorrupted) and (h.walTripEpoch > 1 or h.walProofEpoch > 1))
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("tripEpoch", 1)
        .with("liveProofEpoch", 1)
        .with("restartEpochRegressed", true)
    else
      h' = h

  action stepRestartRegressesEpoch = any { step, restartRegressesEpochUnchecked }

  // Countermodel 3: Corrupt MGV1 WAL Fail-Open — tampered Turso checkpoint boots armed instead of fail-closed.
  action corruptWalFailsOpenUnchecked =
    if (h.walCorrupted)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("breakerTripped", false)
        .with("corruptWalOpened", true)
    else
      h' = h

  action stepCorruptWalFailsOpen = any { step, corruptWalFailsOpenUnchecked }

  // Countermodel 4: Corrupt MGF1 Fact-Graph Fail-Open — tampered `:facts` CAS blob boots armed.
  action corruptFactGraphFailsOpenUnchecked =
    if (h.factGraphCorrupted)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("breakerTripped", false)
        .with("corruptFactGraphOpened", true)
    else
      h' = h

  action stepCorruptFactGraphFailsOpen = any { step, corruptFactGraphFailsOpenUnchecked }

  // Countermodel 5: Cedar Permit Overrides Forbid — matching VerifiedHuman permit bypasses ASCENT forbid.
  action cedarForbidBypassedByPermitUnchecked =
    if (h.liveFenced)
      h' = h.with("cedarForbidBypassed", true)
    else
      h' = h

  action stepCedarForbidBypassedByPermit = any { step, cedarForbidBypassedByPermitUnchecked }

  // Countermodel 6: Retracted Dynamic Derivation Commits — invalidated Bi-Temporal TM conclusion still commits.
  action retractedDerivationCommitsUnchecked =
    if (h.dynamicDerivationRetracted)
      h' = h.with("retractedDerivationCommitted", true)
    else
      h' = h

  action stepRetractedDerivationCommits = any { step, retractedDerivationCommitsUnchecked }

  // Countermodel 7: Scope-Unless Sibling Exfil Bypass — un-declassified external sibling bypasses
  // `forbid (principal, action, resource in Marlin::Scope::"external") unless { context.declassified == true }`.
  action scopeUnlessBypassSiblingExfilUnchecked =
    if (h.liveFenced)
      h' = h.with("scopeUnlessBypassed", true)
    else
      h' = h

  action stepScopeUnlessBypassSiblingExfil = any { step, scopeUnlessBypassSiblingExfilUnchecked }

  // Countermodel 8: Missing Context Key Fail-Open — omitted `context.<k>` satisfies `when`/`!=`/`unless`.
  action missingContextKeyBypassesConditionUnchecked =
    if (h.liveFenced)
      h' = h.with("missingContextBypassed", true)
    else
      h' = h

  action stepMissingContextKeyBypassesCondition = any { step, missingContextKeyBypassesConditionUnchecked }

  run durableRestartRecoveryTest =
    init
      .then(applyAscentIncidentAndPersist)
      .then(restartAndRecoverDurable)
      .then(all {
        assert(h.breakerTripped),
        assert(h.liveFenced),
        assert(h.liveProofEpoch == 2),
        assert(safety),
        h' = h
      })
      .then(rearmBreakerAs(false, true))
      .then(syncWalCheckpoint)
      .then(restartAndRecoverDurable)
      .then(all {
        assert(not(h.breakerTripped)),
        assert(h.tripEpoch == 2),
        assert(h.liveProofEpoch == 2),
        assert(safety),
        h' = h
      })
      .then(tamperFactGraphOnDisk)
      .then(restartAndRecoverDurable)
      .then(all {
        assert(h.breakerTripped),
        assert(h.liveFenced),
        assert(safety),
        h' = h
      })

  run restartClearsEStopCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(restartClearsEStopUnchecked).then(checkUnsafe)

  run restartRegressesEpochCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(restartRegressesEpochUnchecked).then(checkUnsafe)

  run corruptWalFailsOpenCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(tamperWalOnDisk).then(corruptWalFailsOpenUnchecked).then(checkUnsafe)

  run corruptFactGraphFailsOpenCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(tamperFactGraphOnDisk).then(corruptFactGraphFailsOpenUnchecked).then(checkUnsafe)

  run cedarForbidBypassedCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(cedarForbidBypassedByPermitUnchecked).then(checkUnsafe)

  run retractedDerivationCommitsCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(retractedDerivationCommitsUnchecked).then(checkUnsafe)

  run scopeUnlessBypassCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(scopeUnlessBypassSiblingExfilUnchecked).then(checkUnsafe)

  run missingContextBypassCountermodelTest =
    init.then(applyAscentIncidentAndPersist).then(missingContextKeyBypassesConditionUnchecked).then(checkUnsafe)
}
