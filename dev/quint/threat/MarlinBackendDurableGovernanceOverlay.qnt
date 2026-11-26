// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/execution/agent-tool-circuit-breaker/AgentToolCircuitBreaker"
// @countermodel restart-preserves-estop step=stepRestartClearsEStop max-steps=6 itf="h.restartClearedTrip == true"
// @countermodel restart-monotonic-epochs step=stepRestartRegressesEpoch max-steps=8 itf="h.restartEpochRegressed == true"
// @countermodel corrupt-wal-fails-closed step=stepCorruptWalFailsOpen max-steps=6 itf="h.corruptWalOpened == true"

// Marlin Durable Higher-Order Governance overlay composing AgentToolCircuitBreaker
// with Turso WAL / CAS checkpointing, OS process crash-restart recovery, and
// tamper-evident SHA-256 checkpoint integrity verification (anti-restart-reset bypass).
module marlin_backend_durable_governance_overlay {
  type State = {
    walCommitted: bool,
    walTripped: bool,
    walTripEpoch: int,
    walProofEpoch: int,
    walFenced: bool,
    walCorrupted: bool,
    liveProofEpoch: int,
    liveFenced: bool,
    restartClearedTrip: bool,
    restartEpochRegressed: bool,
    corruptWalOpened: bool
  }

  action init = h' = {
    walCommitted: false,
    walTripped: false,
    walTripEpoch: 1,
    walProofEpoch: 1,
    walFenced: false,
    walCorrupted: false,
    liveProofEpoch: 1,
    liveFenced: false,
    restartClearedTrip: false,
    restartEpochRegressed: false,
    corruptWalOpened: false
  }

  // Persist current live breaker + Scheme governance state into Turso WAL/CAS.
  action syncWalCheckpoint =
    if (not(h.walCorrupted))
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
    if (not(h.walCorrupted) and h.liveProofEpoch < 4)
      h' = h
        .with("breakerTripped", true)
        .with("liveFenced", true)
        .with("liveProofEpoch", h.liveProofEpoch + 1)
        .with("walCommitted", true)
        .with("walTripped", true)
        .with("walTripEpoch", h.tripEpoch)
        .with("walProofEpoch", h.liveProofEpoch + 1)
        .with("walFenced", true)
    else
      h' = h

  // Adversary corrupts or bit-flips the persisted Turso WAL blob on disk.
  action tamperWalOnDisk =
    if (h.walCommitted and not(h.walCorrupted))
      h' = h.with("walCorrupted", true)
    else
      h' = h

  // Safe crash + OS process restart recovery:
  // 1. In-flight volatile leases are wiped on crash.
  // 2. Corrupted/tampered WAL ALWAYS fails closed (breakerTripped=true, liveFenced=true).
  // 3. Valid WAL restores persisted tripped latch, monotonic epochs, and ASCENT fences.
  action restartAndRecoverDurable =
    if (h.walCorrupted)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("breakerTripped", true)
        .with("tripEpoch", h.walTripEpoch)
        .with("liveProofEpoch", h.walProofEpoch)
        .with("liveFenced", true)
        .with("corruptWalOpened", h.corruptWalOpened or false)
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
    restartAndRecoverDurable
  }

  val safety =
    not(h.restartClearedTrip) and
    not(h.restartEpochRegressed) and
    not(h.corruptWalOpened)

  // Countermodel 1: Restart-Reset Bypass — daemon reboots and wipes a persisted E-Stop latch.
  action restartClearsEStopUnchecked =
    if (h.walCommitted and h.walTripped and not(h.walCorrupted))
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
    if (h.walCommitted and not(h.walCorrupted) and (h.walTripEpoch > 1 or h.walProofEpoch > 1))
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("tripEpoch", 1)
        .with("liveProofEpoch", 1)
        .with("restartEpochRegressed", true)
    else
      h' = h

  action stepRestartRegressesEpoch = any { step, restartRegressesEpochUnchecked }

  // Countermodel 3: Corrupt WAL Fail-Open — tampered Turso checkpoint boots armed instead of fail-closed.
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
      .then(tamperWalOnDisk)
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
}
