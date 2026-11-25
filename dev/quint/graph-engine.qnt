// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-GRAPH-R01/R02/R03/R04. Session-free TaskActor lifecycle, C4 steering
// join convergence gate, monotonic fencing-epoch anti-split-brain receipt
// reconciliation, and detached-scope write isolation.
module graphEngineTaskActorSafety {
  type State = {
    // 0=openFocus, 1=running, 2=waitingSteering, 3=settled, 4=failed
    status: int,
    // 0=project, 1=detached
    scopeMode: int,
    fencingEpoch: int,
    appliedEpoch: int,
    pendingSteering: int,
    staleReceiptAccepted: bool,
    prematureJoinAdvance: bool,
    detachedCommittedProject: bool,
    mutatedAfterTerminal: bool
  }
  var s: State

  def isTerminal(st: int): bool = st == 3 or st == 4

  action init = s' = {
    status: 0,
    scopeMode: 0,
    fencingEpoch: 1,
    appliedEpoch: 0,
    pendingSteering: 0,
    staleReceiptAccepted: false,
    prematureJoinAdvance: false,
    detachedCommittedProject: false,
    mutatedAfterTerminal: false
  }

  action startDetached = all {
    s.status == 0,
    s' = s.with("scopeMode", 1).with("status", 1)
  }

  action promoteFocus = all {
    s.status == 0,
    s' = s.with("status", 1)
  }

  action anchorSteeringBranch = all {
    s.status == 1 or s.status == 2,
    s.pendingSteering < 3,
    s.fencingEpoch < 8,
    s' = s
      .with("status", 2)
      .with("pendingSteering", s.pendingSteering + 1)
      .with("fencingEpoch", s.fencingEpoch + 1)
  }

  action completeSteeringBranch = all {
    s.status == 2,
    s.pendingSteering > 0,
    s.fencingEpoch < 8,
    val nextPending = s.pendingSteering - 1
    val nextStatus = if (nextPending == 0) 1 else 2
    s' = s
      .with("pendingSteering", nextPending)
      .with("status", nextStatus)
      .with("appliedEpoch", s.fencingEpoch)
      .with("fencingEpoch", s.fencingEpoch + 1)
  }

  action reconcileReceipt(receiptEpoch: int, receiptKind: int): bool = all {
    receiptEpoch >= 1,
    receiptEpoch <= 8,
    // receiptKind: 0=ok, 1=settled, 2=failed
    if (isTerminal(s.status) or receiptEpoch != s.fencingEpoch) {
      s' = s
    } else if (receiptKind == 0) {
      val nextStatus = if (s.pendingSteering > 0) 2 else 1
      s' = s
        .with("appliedEpoch", receiptEpoch)
        .with("fencingEpoch", if (s.fencingEpoch < 8) s.fencingEpoch + 1 else s.fencingEpoch)
        .with("status", nextStatus)
    } else if (receiptKind == 1) {
      if (s.pendingSteering > 0 or s.scopeMode == 1) {
        // Cannot settle with pending steering or detached project commit.
        s' = s
      } else {
        s' = s
          .with("appliedEpoch", receiptEpoch)
          .with("fencingEpoch", if (s.fencingEpoch < 8) s.fencingEpoch + 1 else s.fencingEpoch)
          .with("status", 3)
      }
    } else {
      s' = s
        .with("appliedEpoch", receiptEpoch)
        .with("fencingEpoch", if (s.fencingEpoch < 8) s.fencingEpoch + 1 else s.fencingEpoch)
        .with("status", 4)
    }
  }

  action step = any {
    startDetached,
    promoteFocus,
    anchorSteeringBranch,
    completeSteeringBranch,
    nondet ep = oneOf(Set(1, 2, 3, 4, 5, 6, 7, 8))
    nondet rk = oneOf(Set(0, 1, 2))
    reconcileReceipt(ep, rk),
  }

  def safe = all {
    s.fencingEpoch >= 1,
    s.appliedEpoch <= s.fencingEpoch,
    s.pendingSteering >= 0,
    not(s.staleReceiptAccepted),
    not(s.prematureJoinAdvance),
    not(s.detachedCommittedProject),
    not(s.mutatedAfterTerminal),
    // Settled actors must never retain pending steering branches or detached project commits.
    if (s.status == 3) (s.pendingSteering == 0 and s.scopeMode == 0) else true,
  }

  // Negative controls: each mutation MUST violate `safe`.
  action acceptStaleReceipt = all {
    not(isTerminal(s.status)),
    s.fencingEpoch > 1,
    s' = s
      .with("appliedEpoch", s.fencingEpoch - 1)
      .with("staleReceiptAccepted", true)
  }

  action advancePastJoinWithPendingSteering = all {
    s.pendingSteering > 0,
    s' = s.with("status", 3).with("prematureJoinAdvance", true)
  }

  action commitDetachedToProject = all {
    s.scopeMode == 1,
    not(isTerminal(s.status)),
    s' = s.with("status", 3).with("detachedCommittedProject", true)
  }

  action mutateAfterTerminal = all {
    isTerminal(s.status),
    s' = s.with("status", 1).with("mutatedAfterTerminal", true)
  }

  action staleReceiptAcceptMutation = any { step, acceptStaleReceipt }
  action prematureJoinAdvanceMutation = any { step, advancePastJoinWithPendingSteering }
  action detachedProjectCommitMutation = any { step, commitDetachedToProject }
  action postTerminalMutation = any { step, mutateAfterTerminal }

  run staleEpochReceiptIsRejectedTest =
    init
      .then(promoteFocus)
      .then(anchorSteeringBranch)
      .then(reconcileReceipt(1, 0))
      .expect(safe and s.fencingEpoch == 2 and s.appliedEpoch == 0 and s.status == 2)

  run c4SteeringBlocksSettlementUntilConvergedTest =
    init
      .then(promoteFocus)
      .then(anchorSteeringBranch)
      .then(reconcileReceipt(2, 1))
      .expect(safe and s.status == 2 and s.pendingSteering == 1)
      .then(completeSteeringBranch)
      .then(reconcileReceipt(3, 1))
      .expect(safe and s.status == 3 and s.pendingSteering == 0)

  run detachedScopeBlocksProjectSettlementTest =
    init
      .then(startDetached)
      .then(reconcileReceipt(1, 1))
      .expect(safe and s.scopeMode == 1 and s.status == 1)
}

// MAR-GRAPH-R05. Pinned-Index Task Workbench tab projection invariants:
// slot 0 is always pinned to TaskActorIndex, active tasks and read-only peeks
// stay strictly disjoint, and settled actors leave the active set cleanly.
module graphEngineWorkbenchTabSafety {
  type TabState = {
    indexPinned: bool,
    activeTasks: Set[int],
    peekTasks: Set[int],
    settledTasks: Set[int],
    focusedTask: int
  }
  var t: TabState

  action init = t' = {
    indexPinned: true,
    activeTasks: Set(),
    peekTasks: Set(),
    settledTasks: Set(),
    focusedTask: 0
  }

  action promoteTask(tid: int): bool = all {
    tid.in(Set(1, 2, 3, 4)),
    not(tid.in(t.settledTasks)),
    t' = t
      .with("activeTasks", t.activeTasks.union(Set(tid)))
      .with("peekTasks", t.peekTasks.exclude(Set(tid)))
      .with("focusedTask", tid)
  }

  action settleTask(tid: int): bool = all {
    tid.in(t.activeTasks),
    val nextActive = t.activeTasks.exclude(Set(tid))
    val nextFocus = if (t.focusedTask == tid) 0 else t.focusedTask
    t' = t
      .with("activeTasks", nextActive)
      .with("settledTasks", t.settledTasks.union(Set(tid)))
      .with("focusedTask", nextFocus)
  }

  action openReadOnlyPeek(tid: int): bool = all {
    tid.in(t.settledTasks),
    not(tid.in(t.activeTasks)),
    t' = t.with("peekTasks", t.peekTasks.union(Set(tid)))
  }

  action closePeek(tid: int): bool = all {
    tid.in(t.peekTasks),
    t' = t.with("peekTasks", t.peekTasks.exclude(Set(tid)))
  }

  action step = any {
    nondet tid = oneOf(Set(1, 2, 3, 4))
    promoteTask(tid),
    nondet tid = oneOf(Set(1, 2, 3, 4))
    settleTask(tid),
    nondet tid = oneOf(Set(1, 2, 3, 4))
    openReadOnlyPeek(tid),
    nondet tid = oneOf(Set(1, 2, 3, 4))
    closePeek(tid),
  }

  def safe = all {
    t.indexPinned,
    t.activeTasks.intersect(t.peekTasks) == Set(),
    t.activeTasks.intersect(t.settledTasks) == Set(),
    if (t.focusedTask != 0) t.focusedTask.in(t.activeTasks) else true,
  }

  // Negative controls: each mutation MUST violate `safe`.
  action unpinIndex = all {
    t.indexPinned,
    t' = t.with("indexPinned", false)
  }

  action overlapPeekWithActive = all {
    1.in(t.activeTasks),
    t' = t.with("peekTasks", t.peekTasks.union(Set(1)))
  }

  action unpinnedIndexMutation = any { step, unpinIndex }
  action peekOverlapMutation = any { step, overlapPeekWithActive }

  run settleMovesActiveToArchivedPeekableTest =
    init
      .then(promoteTask(1))
      .then(settleTask(1))
      .then(openReadOnlyPeek(1))
      .expect(safe and t.activeTasks == Set() and t.peekTasks == Set(1) and t.focusedTask == 0)
}
