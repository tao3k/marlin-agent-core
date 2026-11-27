// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/concurrency/bounded-projection-eviction/BoundedProjectionEviction"

// Marlin MyGoBridgeHost (ui_grpc.rs) & marlin-mygo-host bounded retention overlay:
// enforces bounded artifact/gate/device-result/session projections with reclaimable
// eviction and zero orphaned side-map entries.
module marlin_mygo_bridge_stability_composed {
  type State = { peakBridgeEntries: int, orphanObserved: bool }
  action init = h' = { peakBridgeEntries: 0, orphanObserved: false }
  action observeBridgeBounds = h' = h
    .with("peakBridgeEntries", if (h.entries > h.peakBridgeEntries) h.entries else h.peakBridgeEntries)
    .with("orphanObserved", h.orphanObserved or h.orphanedSideMap > 0)
  action step = any { observeBridgeBounds }
  val safety = not(h.orphanObserved) and h.peakBridgeEntries <= 2 and h.orphanedSideMap == 0
  run marlinBridgeBoundedEvictionTest = init
    .then(admitEntry)
    .then(admitEntry)
    .then(observeBridgeBounds)
    .then(deactivateEntry)
    .then(admitWithIdleEviction)
    .then(deactivateEntry)
    .then(evictIdleEntry)
    .then(observeBridgeBounds)
    .then(all {
      assert(h.peakBridgeEntries <= 2),
      assert(h.evictions >= 2),
      assert(not(h.orphanObserved)),
      assert(safety),
      h' = h
    })
}
