// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/protocol/chatroom-ingress-governance/ChatroomIngressGovernance"
// @countermodel forged-ingress-auto-trips-breaker step=stepForgedIngressNoAutoTrip max-steps=6 itf="h.forgedIngressNoTrip == true"

// Marlin Rust backend ChatroomIngressGovernance overlay coupling forged external
// chatroom ingress attempts directly to fail-closed ExecutionCircuitBreaker tripping.
module marlin_backend_chatroom_ingress_overlay {
  type State = {
    backendBreakerTripped: bool,
    forgedIngressNoTrip: bool
  }

  action init = h' = {
    backendBreakerTripped: false,
    forgedIngressNoTrip: false
  }

  action rejectForgedIngressAndTripBreaker =
    h' = h.with("backendBreakerTripped", true)

  action humanRearmChatroomBreakerAs(isAgent: bool): bool =
    if (h.backendBreakerTripped and not(isAgent))
      h' = h.with("backendBreakerTripped", false)
    else
      h' = h

  action humanRearmChatroomBreaker = {
    nondet isAgent = Set(false, true).oneOf()
    humanRearmChatroomBreakerAs(isAgent)
  }

  action step = any {
    rejectForgedIngressAndTripBreaker,
    humanRearmChatroomBreaker
  }

  val safety = not(h.forgedIngressNoTrip)

  action rejectForgedIngressWithoutTrip =
    if (not(h.backendBreakerTripped))
      h' = h.with("forgedIngressNoTrip", true)
    else
      h' = h

  action stepForgedIngressNoAutoTrip = any { step, rejectForgedIngressWithoutTrip }

  run forgedIngressAutoTripsAndHumanRearmsTest =
    init
      .then(rejectForgedIngressAndTripBreaker)
      .then(humanRearmChatroomBreakerAs(true))
      .then(humanRearmChatroomBreakerAs(false))
      .then(all {
        assert(not(h.backendBreakerTripped)),
        assert(not(h.forgedIngressNoTrip)),
        assert(safety),
        h' = h
      })

  run forgedIngressNoAutoTripCountermodelTest =
    init.then(rejectForgedIngressWithoutTrip).then(checkUnsafe)
}
