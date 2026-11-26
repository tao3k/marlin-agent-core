// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/execution/agent-tool-circuit-breaker/AgentToolCircuitBreaker"
// @countermodel camel-taint-blocks-side-effect step=stepBackendTaintedCommit max-steps=6 itf="h.taintedSideEffectCommit == true"

// Marlin Rust backend ExecutionCircuitBreaker + LoopProgramRuntimeSideEffectExecutor
// overlay composing upstream AgentToolCircuitBreaker with OS process spawn,
// workspace file write, and CaMeL tainted-control-flow cutoff guarantees.
module marlin_backend_execution_breaker_overlay {
  type State = {
    ifcTaintedControl: bool,
    processSpawns: int,
    fileWrites: int,
    taintedSideEffectCommit: bool
  }

  action init = h' = {
    ifcTaintedControl: false,
    processSpawns: 0,
    fileWrites: 0,
    taintedSideEffectCommit: false
  }

  action markIfcTainted =
    h' = h.with("ifcTaintedControl", true)

  action clearIfcTaint =
    h' = h.with("ifcTaintedControl", false)

  action commitBackendToolProcess =
    if (h.leaseActive and not(h.breakerTripped) and epochCurrent(h.tripEpoch, h.leaseEpoch) and h.budgetUsed < 1 and not(h.ifcTaintedControl))
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("budgetUsed", h.budgetUsed + 1)
        .with("processSpawns", h.processSpawns + 1)
        .with("execWhileTripped", h.execWhileTripped or h.breakerTripped)
        .with("staleLeaseCommit", h.staleLeaseCommit or h.tripEpoch != h.leaseEpoch)
        .with("budgetOverrun", h.budgetOverrun or h.budgetUsed >= 1)
        .with("taintedSideEffectCommit", h.taintedSideEffectCommit or h.ifcTaintedControl)
    else
      h' = h

  action commitBackendFileWrite =
    if (h.leaseActive and not(h.breakerTripped) and epochCurrent(h.tripEpoch, h.leaseEpoch) and h.budgetUsed < 1 and not(h.ifcTaintedControl))
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("budgetUsed", h.budgetUsed + 1)
        .with("fileWrites", h.fileWrites + 1)
        .with("execWhileTripped", h.execWhileTripped or h.breakerTripped)
        .with("staleLeaseCommit", h.staleLeaseCommit or h.tripEpoch != h.leaseEpoch)
        .with("budgetOverrun", h.budgetOverrun or h.budgetUsed >= 1)
        .with("taintedSideEffectCommit", h.taintedSideEffectCommit or h.ifcTaintedControl)
    else
      h' = h

  action step = any {
    markIfcTainted,
    clearIfcTaint,
    commitBackendToolProcess,
    commitBackendFileWrite
  }

  val safety = not(h.taintedSideEffectCommit)

  action commitBackendToolProcessNoIfcGuard =
    if (h.leaseActive and not(h.breakerTripped) and epochCurrent(h.tripEpoch, h.leaseEpoch) and h.budgetUsed < 1)
      h' = h
        .with("leaseActive", false)
        .with("leaseEpoch", 0)
        .with("budgetUsed", h.budgetUsed + 1)
        .with("processSpawns", h.processSpawns + 1)
        .with("taintedSideEffectCommit", h.taintedSideEffectCommit or h.ifcTaintedControl)
    else
      h' = h

  action stepBackendTaintedCommit = any { step, commitBackendToolProcessNoIfcGuard }

  run backendProcessFileAndTaintCutoffTest =
    init
      .then(markIfcTainted)
      .then(acquireToolLease)
      .then(commitBackendToolProcess)
      .then(clearIfcTaint)
      .then(commitBackendToolProcess)
      .then(tripBreaker)
      .then(rearmBreakerAs(false, true))
      .then(acquireToolLease)
      .then(commitBackendFileWrite)
      .then(all {
        assert(h.processSpawns == 1),
        assert(h.fileWrites == 1),
        assert(h.tripEpoch == 2),
        assert(not(h.taintedSideEffectCommit)),
        assert(safety),
        h' = h
      })

  run backendTaintedCommitCountermodelTest =
    init
      .then(markIfcTainted)
      .then(acquireToolLease)
      .then(commitBackendToolProcessNoIfcGuard)
      .then(checkUnsafe)
}
