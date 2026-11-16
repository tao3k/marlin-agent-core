// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
module runtimeProcessOwner {
  // A and B are distinct sessions sharing one operation name. Generation zero
  // denotes absence. C shares A's session but has a different operation.
  // The pipe belongs to A's current or most recently reaped child.
  var state: { closed: bool, a: int, b: int, c: int, lastA: int, held: bool, borrowed: bool,
               task: bool, next: int, done: bool, report: int, reason: str, target: int, event: str }

  action init = state' = {
    closed: false, a: 0, b: 0, c: 0, lastA: 0, held: false, borrowed: false,
    task: false, next: 1, done: false, report: 0, reason: "none", target: 0, event: "init"
  }

  def ownsA = state.a != 0 or state.held
  def drained = not(ownsA) and state.b == 0 and state.c == 0 and not(state.task)

  action startA = all {
    not(state.closed), not(ownsA),
    state' = state.with("a", state.next).with("next", state.next + 1).with("borrowed", false).with("event", "startA")
  }
  action startB = all {
    not(state.closed), state.b == 0,
    state' = state.with("b", state.next).with("next", state.next + 1).with("event", "startB")
  }
  action startC = all {
    not(state.closed), state.c == 0,
    state' = state.with("c", state.next).with("next", state.next + 1).with("event", "startC")
  }
  action duplicateA = all {
    not(state.closed), ownsA,
    state' = state.with("event", "duplicateA")
  }
  action borrowA = all {
    state.a != 0, not(state.borrowed),
    state' = state.with("held", true).with("borrowed", true).with("event", "borrowA")
  }
  action reapA = all {
    state.a != 0,
    state' = state.with("lastA", state.a).with("a", 0).with("event", "reapA")
  }
  action reapB = all {
    state.b != 0, state' = state.with("b", 0).with("event", "reapB")
  }
  action reapC = all {
    state.c != 0, state' = state.with("c", 0).with("event", "reapC")
  }
  action releasePipe = all {
    state.held, state' = state.with("held", false).with("event", "releasePipe")
  }
  action staleA = all {
    state.lastA != 0, state.a > state.lastA, state' = state.with("event", "staleA")
  }
  action observeA = all {
    state.a != 0,
    state' = state.with("report", state.a).with("reason", "deadline").with("event", "observeA")
  }
  action observeHost = all {
    state.a != 0,
    state' = state.with("report", state.a).with("reason", "host").with("event", "observeHost")
  }
  action observeIdle = all {
    state.a != 0,
    state' = state.with("report", state.a).with("reason", "idle").with("event", "observeIdle")
  }
  action applyA = all {
    state.report != 0, state.a != 0,
    state' = if (state.report == state.a and state.reason == "deadline")
      state.with("a", 0).with("lastA", state.a).with("target", state.a).with("event", "applyA")
    else state.with("target", 0).with("event", "applyA")
  }
  action applyB = all {
    state.report != 0, state.b != 0,
    state' = state.with("target", 0).with("event", "applyB")
  }
  action applyC = all {
    state.report != 0, state.c != 0,
    state' = state.with("target", 0).with("event", "applyC")
  }
  action startTask = all {
    not(state.closed), not(state.task),
    state' = state.with("task", true).with("next", state.next + 1).with("event", "startTask")
  }
  action releaseTask = all {
    state.task, state' = state.with("task", false).with("event", "releaseTask")
  }
  action close = all {
    not(state.closed), state' = state.with("closed", true).with("event", "close")
  }
  action rejectTask = all {
    state.closed, state' = state.with("event", "rejectTask")
  }
  action rejectProcess = all {
    state.closed, state' = state.with("event", "rejectProcess")
  }
  action finish = all {
    state.closed, drained, state' = state.with("done", true).with("event", "finish")
  }

  action step = any {
    startA, startB, startC, duplicateA, borrowA, reapA, reapB, reapC, releasePipe,
    staleA, observeA, observeHost, observeIdle, applyA, applyB, applyC,
    startTask, releaseTask, close, rejectTask, rejectProcess, finish
  }
  action badStart = all {
    state.closed, not(ownsA),
    state' = state.with("a", state.next).with("next", state.next + 1).with("event", "startA")
  }
  action badFinish = all {
    state.closed, state.held,
    state' = state.with("done", true).with("event", "finish")
  }
  action badStep = any { step, badStart, badFinish }
  action badWatchdog = all {
    state.report != 0, state.a > state.report, state.reason == "deadline",
    state' = state.with("a", 0).with("lastA", state.a).with("target", state.a).with("event", "applyA")
  }
  action watchdogMutation = any { step, badWatchdog }
  action badHealth = all {
    state.a != 0, state.report == state.a, Set("host", "idle").contains(state.reason),
    state' = state.with("a", 0).with("lastA", state.a).with("target", state.a).with("event", "applyA")
  }
  action healthMutation = any { step, badHealth }

  def safe = all {
    state.closed implies not(Set("startA", "startB", "startC", "startTask").contains(state.event)),
    state.done implies (state.closed and drained),
    (state.a != 0 and state.b != 0) implies state.a != state.b,
    state.a == 0 or state.a > state.lastA,
    (state.event == "applyA" and state.target != 0) implies (state.target == state.report and state.reason == "deadline"),
    Set("applyB", "applyC").contains(state.event) implies state.target == 0,
  }

  run shutdownWaitsForPipeTest = init.then(startA).then(borrowA).then(close)
    .then(rejectTask).then(rejectProcess).then(reapA).then(releasePipe).then(finish)
  run sessionAndGenerationTest = init.then(startA).then(duplicateA).then(startB)
    .then(reapA).then(startA).then(staleA).then(reapB).then(reapA).then(close).then(finish)
  run shutdownWaitsForTaskTest = init.then(startTask).then(close).then(rejectTask)
    .then(releaseTask).then(finish)
  run exitedPipeStillOwnsOperationTest = init.then(startA).then(borrowA)
    .then(reapA).then(duplicateA).then(releasePipe).then(startA).then(reapA)
  run watchdogPreservesReplacementTest = init.then(startA).then(observeA).then(startB)
    .then(applyB).then(startC).then(applyC).then(reapA).then(startA)
    .then(applyA).then(reapA).then(reapB).then(reapC)
  run watchdogOwnerReapsWithPipeTest = init.then(startA).then(borrowA).then(observeA)
    .then(close).then(applyA).then(releasePipe).then(finish)
  run backgroundHealthIsAdvisoryTest = init.then(startA).then(observeHost).then(applyA)
    .then(observeIdle).then(applyA).then(reapA).then(startA).then(applyA)
    .then(observeA).then(applyA)
}
