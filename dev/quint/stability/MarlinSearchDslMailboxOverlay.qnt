// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/concurrency/priority-isolated-bounded-mailbox/PriorityIsolatedBoundedMailbox"
// @verify safety max-steps=12
// Marlin Rust domain overlay for LinkedSearchDslMailbox & AspGatewaySupervisor watch channel in marlin-agent-tui (SM11).
module marlin_search_dsl_mailbox_overlay {
  type State = {
    perTierChannelsIsolated: bool
  }
  action init = h' = { perTierChannelsIsolated: true }
  val safety = h.perTierChannelsIsolated
  run marlinDusaFloodPreservesFastAscentTest =
    init
      .then(submitLo)
      .then(submitLo)
      .then(submitLo)
      .then(submitHi)
      .then(publishProgress)
      .then(publishProgress)
      .then(pollProgress)
      .then(drainOne)
      .then(drainOne)
      .then(drainOne)
      .then(all {
        assert(h.perTierChannelsIsolated),
        assert(h.shedLoCount > 0),
        assert(not(h.hiStarved)),
        assert(h.zombieLo == 0),
        assert(h.progressSlots == 0),
        assert(h.observedWater == h.producedWater),
        assert(safety),
        h' = h
      })
}
