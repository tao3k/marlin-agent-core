// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/identity/delegated-agent-authz/DelegatedAgentAuthz"
// @countermodel step-up-required-for-breaker-rearm step=stepBreakerRearmWithoutStepUp max-steps=6 itf="h.breakerRearmedWithoutStepUp == true"

// Marlin Rust backend DelegatedAgentAuthz overlay coupling RFC 8693 OBO delegation
// and RFC 9470 step-up verification directly to ExecutionCircuitBreaker re-arm control.
module marlin_backend_delegated_authz_overlay {
  type State = {
    backendBreakerTripped: bool,
    breakerRearmedWithoutStepUp: bool
  }

  action init = h' = {
    backendBreakerTripped: false,
    breakerRearmedWithoutStepUp: false
  }

  action tripBackendBreaker =
    if (not(h.backendBreakerTripped))
      h' = h.with("backendBreakerTripped", true)
    else
      h' = h

  action rearmBackendBreakerWithStepUpAs(isAgent: bool): bool =
    if (h.backendBreakerTripped and not(isAgent) and h.humanSignedIn and h.emailVerified and h.stepUpFresh)
      h' = h
        .with("backendBreakerTripped", false)
        .with("stepUpFresh", false)
        .with("breakerRearmedWithoutStepUp", h.breakerRearmedWithoutStepUp or isAgent or not(h.humanSignedIn) or not(h.emailVerified) or not(h.stepUpFresh))
    else
      h' = h

  action rearmBackendBreakerWithStepUp = {
    nondet isAgent = Set(false, true).oneOf()
    rearmBackendBreakerWithStepUpAs(isAgent)
  }

  action step = any {
    tripBackendBreaker,
    rearmBackendBreakerWithStepUp
  }

  val safety = not(h.breakerRearmedWithoutStepUp)

  action rearmBackendBreakerNoStepUp(isAgent: bool): bool =
    if (h.backendBreakerTripped)
      h' = h
        .with("backendBreakerTripped", false)
        .with("breakerRearmedWithoutStepUp", h.breakerRearmedWithoutStepUp or isAgent or not(h.humanSignedIn) or not(h.emailVerified) or not(h.stepUpFresh))
    else
      h' = h

  action stepBreakerRearmWithoutStepUp = any {
    step,
    {
      nondet isAgent = Set(false, true).oneOf()
      rearmBackendBreakerNoStepUp(isAgent)
    }
  }

  run stepUpRequiredToRearmBackendBreakerTest =
    init
      .then(tripBackendBreaker)
      .then(rearmBackendBreakerWithStepUpAs(true))
      .then(rearmBackendBreakerWithStepUpAs(false))
      .then(performStepUp)
      .then(rearmBackendBreakerWithStepUpAs(false))
      .then(all {
        assert(not(h.backendBreakerTripped)),
        assert(not(h.breakerRearmedWithoutStepUp)),
        assert(safety),
        h' = h
      })

  run breakerRearmWithoutStepUpCountermodelTest =
    init.then(tripBackendBreaker).then(rearmBackendBreakerNoStepUp(false)).then(checkUnsafe)
}
