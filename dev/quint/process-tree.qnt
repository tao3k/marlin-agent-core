// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-PROC-R03/R04: one leader, one same-group descendant and one
// inherited output pipe; descendant exit and EOF are observed together.
module processTree {
  var leader: str
  var descendant: str
  var pipe: str
  var task: str
  var owner: str
  var queued: bool
  var b: str
  action init = all { leader' = "pending", descendant' = "none", pipe' = "closed", task' = "pending", owner' = "none", queued' = false, b' = "pending" }
  action startA = all { leader == "pending", leader' = "running", descendant' = "running", pipe' = "open", task' = "running", owner' = "a", queued' = queued, b' = b }
  action cancelA = all { task == "running", task' = "cancelled", leader' = leader, descendant' = descendant, pipe' = pipe, owner' = owner, queued' = queued, b' = b }
  action exitLeader = all { task == "cancelled", leader == "running", leader' = "exited", descendant' = descendant, pipe' = pipe, task' = task, owner' = owner, queued' = queued, b' = b }
  action queueB = all { owner == "a", not(queued), queued' = true, leader' = leader, descendant' = descendant, pipe' = pipe, task' = task, owner' = owner, b' = b }
  action drain = all { leader == "exited", descendant == "running", descendant' = "exited", pipe' = "closed", owner' = if (queued) "b" else "none", b' = if (queued) "running" else b, queued' = false, task' = task, leader' = leader }
  action startB = all { owner == "none", leader == "exited", b == "pending", b' = "running", owner' = "b", queued' = queued, leader' = leader, descendant' = descendant, pipe' = pipe, task' = task }
  action exitB = all { b == "running", b' = "exited", owner' = "none", queued' = queued, leader' = leader, descendant' = descendant, pipe' = pipe, task' = task }
  action step = any { startA, cancelA, exitLeader, queueB, drain, startB, exitB }
  // Negative control: leader exit incorrectly releases the live group.
  action badExitLeader = all { task == "cancelled", leader == "running", leader' = "exited", descendant' = descendant, pipe' = pipe, task' = task, owner' = "none", queued' = queued, b' = b }
  action badStep = any { startA, cancelA, badExitLeader, queueB, drain, startB, exitB }
  def safe = all { (descendant == "running" or pipe == "open") implies owner == "a", b == "running" implies owner == "b" }
  run queuedAfterLeaderExitTest = init.then(startA).then(cancelA).then(exitLeader).then(queueB).then(drain).then(exitB)
  run queuedBeforeLeaderExitTest = init.then(startA).then(queueB).then(cancelA).then(exitLeader).then(drain).then(exitB)
}
