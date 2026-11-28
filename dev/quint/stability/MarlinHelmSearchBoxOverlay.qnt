// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/concurrency/search-query-metamorphic-algebra/SearchQueryMetamorphicAlgebra"
// @verify safety max-steps=12
// Marlin Rust TUI domain overlay for Modern Helm search-box algebra & viewport hit-testing (SM10).
module marlin_helm_search_box_overlay {
  type State = {
    sexprProtected: bool
  }
  action init = h' = { sexprProtected: true }
  val safety = h.sexprProtected
  run marlinHelmSearchBoxSafeTest =
    init
      .then(applyQualifier)
      .then(applyScopeReplace)
      .then(applyScopeReplace)
      .then(addNegationFilter)
      .then(clickClippedTab)
      .then(resetQuery)
      .then(all {
        assert(h.sexprProtected),
        assert(h.effectiveScopeEpoch == h.activeScopeEpoch),
        assert(not(h.phantomTabHit)),
        assert(safety),
        h' = h
      })
}
