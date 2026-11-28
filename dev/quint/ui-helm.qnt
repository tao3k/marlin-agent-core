// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-HELM-R01/R02/R03. Non-blocking Modern Helm epoch fencing, stale async
// result rejection, bounded virtual viewport, and mark-pruning custody.
module uiHelmEpochSafety {
  type State = {
    open: bool,
    epoch: int,
    appliedEpoch: int,
    candidates: Set[int],
    marks: Set[int],
    selected: int,
    viewportOffset: int,
    viewportHeight: int,
    staleApplied: bool,
    appliedWhileClosed: bool
  }
  var s: State

  action init = s' = {
    open: true,
    epoch: 1,
    appliedEpoch: 0,
    candidates: Set(),
    marks: Set(),
    selected: 0,
    viewportOffset: 0,
    viewportHeight: 2,
    staleApplied: false,
    appliedWhileClosed: false
  }

  action typeAsyncQuery = all {
    s.open,
    s.epoch < 6,
    s' = s.with("epoch", s.epoch + 1)
  }

  action typeLocalOrderlessFilter(nextCands: Set[int]): bool = all {
    s.open,
    nextCands.subseteq(s.candidates),
    val nextMarks = s.marks.intersect(nextCands)
    s' = s.with("candidates", nextCands).with("marks", nextMarks)
  }

  action applyAsyncResults(resultEpoch: int, nextCands: Set[int]): bool = all {
    resultEpoch >= 1,
    resultEpoch <= 6,
    if (s.open and resultEpoch == s.epoch) {
      val nextMarks = s.marks.intersect(nextCands)
      s' = s
        .with("appliedEpoch", resultEpoch)
        .with("candidates", nextCands)
        .with("marks", nextMarks)
        .with("selected", 0)
        .with("viewportOffset", 0)
    } else {
      s' = s
    }
  }

  action toggleMark(cid: int): bool = all {
    s.open,
    cid.in(s.candidates),
    val nextMarks = if (cid.in(s.marks)) s.marks.exclude(Set(cid)) else s.marks.union(Set(cid))
    s' = s.with("marks", nextMarks)
  }

  action cancelSession = all {
    s.open,
    s' = s.with("open", false)
  }

  action reopenSession = all {
    not(s.open),
    s.epoch < 6,
    s' = s
      .with("open", true)
      .with("epoch", s.epoch + 1)
      .with("candidates", Set())
      .with("marks", Set())
      .with("selected", 0)
      .with("viewportOffset", 0)
  }

  action step = any {
    typeAsyncQuery,
    nondet subset = oneOf(Set(Set(), Set(1), Set(2), Set(1, 2)))
    typeLocalOrderlessFilter(subset),
    nondet re = oneOf(Set(1, 2, 3, 4, 5, 6))
    nondet cands = oneOf(Set(Set(1, 2), Set(2, 3), Set(1, 2, 3)))
    applyAsyncResults(re, cands),
    nondet cid = oneOf(Set(1, 2, 3))
    toggleMark(cid),
    cancelSession,
    reopenSession,
  }

  def safe = all {
    s.appliedEpoch <= s.epoch,
    s.marks.subseteq(s.candidates),
    s.viewportOffset >= 0,
    s.viewportHeight > 0,
    not(s.staleApplied),
    not(s.appliedWhileClosed),
  }

  // Negative controls: each mutation must violate `safe`.
  action applyStaleResult = all {
    s.open,
    s.epoch > 1,
    s' = s
      .with("appliedEpoch", s.epoch - 1)
      .with("candidates", Set(1))
      .with("staleApplied", true)
  }

  action applyResultsWhenClosed = all {
    not(s.open),
    s' = s.with("candidates", Set(1, 2)).with("appliedWhileClosed", true)
  }

  action keepDanglingMark = all {
    s.open,
    1.in(s.marks),
    s' = s.with("candidates", Set(2))
  }

  action staleResultOverwriteMutation = any { step, applyStaleResult }
  action applyWhileClosedMutation = any { step, applyResultsWhenClosed }
  action danglingMarkMutation = any { step, keepDanglingMark }

  run staleAsyncResponseIsDroppedTest =
    init
      .then(typeAsyncQuery)
      .then(applyAsyncResults(1, Set(1, 2)))
      .expect(safe and s.epoch == 2 and s.appliedEpoch == 0 and s.candidates == Set())

  run freshAsyncResponseAppliesAndPrunesMarksTest =
    init
      .then(applyAsyncResults(1, Set(1, 2)))
      .then(toggleMark(1))
      .then(typeAsyncQuery)
      .then(applyAsyncResults(2, Set(2, 3)))
      .expect(safe and s.appliedEpoch == 2 and s.marks == Set() and s.candidates == Set(2, 3))

  run closedSessionIgnoresLateResultsTest =
    init
      .then(cancelSession)
      .then(applyAsyncResults(1, Set(1, 2)))
      .expect(safe and not(s.open) and s.candidates == Set())
}
