// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Specification, not a projection of the current Rust implementation.
// One permit, two tasks, and independently observed child lifetimes.
module processQuota {
  var owner: str
  var a: str
  var b: str
  var taskA: str
  var queuedB: bool

  action init = all { owner' = "none", a' = "pending", b' = "pending", taskA' = "pending", queuedB' = false }

  action startA = all {
    owner == "none", a == "pending",
    owner' = "a", a' = "running", b' = b, taskA' = "running", queuedB' = queuedB,
  }
  action cancelA = all {
    a == "running", a' = "draining", owner' = owner, b' = b, taskA' = "cancelled", queuedB' = queuedB,
  }
  action exitA = all {
    a == "draining", a' = "exited", owner' = if (queuedB) "b" else "none", b' = if (queuedB) "running" else b, taskA' = taskA, queuedB' = false,
  }
  action queueB = all {
    owner == "a", b == "pending", not(queuedB),
    owner' = owner, a' = a, b' = b, taskA' = taskA, queuedB' = true,
  }
  action startB = all {
    owner == "none", b == "pending",
    owner' = "b", b' = "running", a' = a, taskA' = taskA, queuedB' = false,
  }
  action exitB = all {
    b == "running", b' = "exited", owner' = "none", a' = a, taskA' = taskA, queuedB' = queuedB,
  }
  action step = any { startA, cancelA, exitA, queueB, startB, exitB }

  // Mutation: returning the permit at cancellation while A remains alive.
  action badCancelA = all {
    a == "running", a' = "draining", owner' = "none", b' = b, taskA' = "cancelled", queuedB' = queuedB,
  }
  action badStep = any { startA, badCancelA, exitA, queueB, startB, exitB }

  // Safety only: no fairness or eventual-cleanup claim is made.
  def safe = all {
    (a == "running" or a == "draining") implies owner == "a",
    b == "running" implies owner == "b",
    a == "draining" implies taskA == "cancelled",
    not ((a == "running" or a == "draining") and b == "running"),
  }
  action observe(expectedOwner, expectedA, expectedB) = all {
    assert(owner == expectedOwner and a == expectedA and b == expectedB and
      (expectedA != "draining" or taskA == "cancelled")),
    owner' = owner, a' = a, b' = b, taskA' = taskA, queuedB' = queuedB,
  }
  run cancellationRetainsPermitTest = init.then(startA).then(cancelA)
    .then(queueB).then(observe("a", "draining", "pending"))
    .then(exitA).then(observe("b", "exited", "running"))
  run bFirstTest = init.then(startB).then(exitB).then(startA).then(cancelA).then(exitA)
  run queueBeforeCancellationTest = init.then(startA).then(queueB).then(cancelA).then(exitA).then(exitB)
}
