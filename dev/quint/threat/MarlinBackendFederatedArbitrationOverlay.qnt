// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/federated-arbitration-liveness/FederatedArbitrationLiveness"
// @countermodel marlin-b13-unbounded-retry step=stepMarlinUnboundedRetry max-steps=6 itf="h.unboundedRetryViolations > 0"
// @countermodel marlin-b13-deadlock-retry step=stepMarlinDeadlockRetry max-steps=6 itf="h.deadlockRetryViolations > 0"
// @countermodel marlin-b13-terminal-reentry step=stepMarlinTerminalReentry max-steps=6 itf="h.terminalReentryViolations > 0"

// Marlin B13 Federated Arbitration Liveness overlay composing
// FederatedArbitrationLiveness with marlin-agent-kernel's
// FederatedArbitrationLivenessGate (bounded retry budget, wait-for deadlock
// cycle latching, forced escalation, and absorbing terminal phases).
module marlin_backend_federated_arbitration_overlay {
  type State = {
    receiptPinned: bool
  }

  action init = h' = {
    receiptPinned: false
  }

  action admitMarlinArbitrationSafe =
    if (not(h.terminated) and not(h.deadlockDetected) and h.attempts < 2)
      h' = h
        .with("terminated", true)
        .with("admitted", true)
        .with("receiptPinned", true)
    else
      h' = h

  action escalateMarlinArbitrationSafe =
    if (not(h.terminated) and (h.deadlockDetected or h.attempts >= 2))
      h' = h
        .with("terminated", true)
        .with("escalated", true)
        .with("receiptPinned", true)
    else
      h' = h

  action step = any {
    detectDeadlockCycle,
    attemptAutoRetry,
    escalateMarlinArbitrationSafe,
    admitMarlinArbitrationSafe
  }

  val safety = (
    h.unboundedRetryViolations == 0 and
    h.deadlockRetryViolations == 0 and
    h.terminalReentryViolations == 0 and
    not(h.escalated and h.admitted)
  )

  action retryMarlinOverBudget =
    if (h.attempts < 3 and h.attempts >= 2 and not(h.deadlockDetected) and not(h.terminated))
      h' = h
        .with("attempts", h.attempts + 1)
        .with("unboundedRetryViolations", h.unboundedRetryViolations + 1)
    else
      h' = h

  action stepMarlinUnboundedRetry = any { step, retryMarlinOverBudget }

  action retryMarlinInDeadlock =
    if (h.attempts < 3 and h.attempts < 2 and h.deadlockDetected and not(h.terminated))
      h' = h
        .with("attempts", h.attempts + 1)
        .with("deadlockRetryViolations", h.deadlockRetryViolations + 1)
    else
      h' = h

  action stepMarlinDeadlockRetry = any { step, retryMarlinInDeadlock }

  action retryMarlinAfterTerminal =
    if (h.attempts < 3 and h.attempts < 2 and not(h.deadlockDetected) and h.terminated)
      h' = h
        .with("attempts", h.attempts + 1)
        .with("terminalReentryViolations", h.terminalReentryViolations + 1)
    else
      h' = h

  action stepMarlinTerminalReentry = any { step, retryMarlinAfterTerminal }
}
