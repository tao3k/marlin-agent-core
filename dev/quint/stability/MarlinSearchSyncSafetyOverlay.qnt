// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/concurrency/singleflight-cancellation-safety/SingleFlightCancellationSafety"
// @verify safety max-steps=12
// Marlin Rust domain overlay for DebouncedSyncCoordinator in marlin-agent-tooling (SM9).
module marlin_search_sync_safety_overlay {
  type State = {
    dropGuardArmed: bool
  }
  action init = h' = { dropGuardArmed: true }
  val safety = h.dropGuardArmed
  run marlinSearchSyncCancelSafeTest =
    init
      .then(acquireLeader)
      .then(coalesceWaiter)
      .then(cancelLeader)
      .then(acquireLeader)
      .then(coalesceWaiter)
      .then(completeLeader)
      .then(all {
        assert(h.dropGuardArmed),
        assert(not(h.inFlight)),
        assert(h.stuckWaiters == 0),
        assert(h.armedWaiters == 0),
        assert(safety),
        h' = h
      })
}
