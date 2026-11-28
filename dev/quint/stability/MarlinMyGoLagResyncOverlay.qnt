// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/concurrency/stream-lag-resync-coalescing/StreamLagResyncCoalescing"

// Marlin MyGoBridgeHost watch_workbench / watch_graph_trajectory lag-resync
// continuity overlay: slow UI subscribers experiencing Tokio broadcast::Lagged
// MUST resync the latest generation snapshot instead of silently closing the stream.
module marlin_mygo_lag_resync_composed {
  type State = { lagResyncVerified: bool }
  action init = h' = { lagResyncVerified: false }
  action observeLagResync = h' = h
    .with("lagResyncVerified", h.lagResyncVerified or (h.streamAlive and not(h.lagged) and h.clientGen == h.serverGen and h.serverGen >= 3))
  action step = any { observeLagResync }
  val safety = h.streamAlive and h.uncoalescedFlushes == 0 and h.clientGen <= h.serverGen
  run marlinWatchStreamLagResyncTest = init
    .then(emitDelta)
    .then(emitDelta)
    .then(flushCheckpoint)
    .then(emitDelta)
    .then(resyncOnLag)
    .then(observeLagResync)
    .then(all {
      assert(h.streamAlive),
      assert(h.lagResyncVerified),
      assert(h.uncoalescedFlushes == 0),
      assert(safety),
      h' = h
    })
}
