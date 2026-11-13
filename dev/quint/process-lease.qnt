// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-PROC-R05/R06/R07: strict containment admission and an externally held
// managed stdin lease. Process exit and pipe closure are independent events.
module processLease {
  var group: str
  var task: str
  var pipe: str
  var owner: str
  var queued: bool
  var b: str
  var strict: str
  action init = all { group' = "pending", task' = "pending", pipe' = "none", owner' = "none", queued' = false, b' = "pending", strict' = "ready" }
  action rejectStrict = all { group == "pending", strict == "ready", strict' = "rejected", group' = group, task' = task, pipe' = pipe, owner' = owner, queued' = queued, b' = b }
  action startA = all { group == "pending", group' = "running", task' = "running", owner' = "a", pipe' = pipe, queued' = queued, b' = b, strict' = strict }
  action borrowPipe = all { group == "running", pipe == "none", pipe' = "held", group' = group, task' = task, owner' = owner, queued' = queued, b' = b, strict' = strict }
  action cancelA = all { task == "running", pipe != "none", task' = "cancelled", group' = group, pipe' = pipe, owner' = owner, queued' = queued, b' = b, strict' = strict }
  action queueB = all { owner == "a", not(queued), queued' = true, group' = group, task' = task, pipe' = pipe, owner' = owner, b' = b, strict' = strict }
  action exitGroup = all { task == "cancelled", group == "running", group' = "exited", owner' = if (pipe == "held") "a" else if (queued) "b" else "none", b' = if (pipe != "held" and queued) "running" else b, queued' = if (pipe == "held") queued else false, task' = task, pipe' = pipe, strict' = strict }
  action closePipe = all { pipe == "held", pipe' = "closed", owner' = if (group == "exited") (if (queued) "b" else "none") else owner, b' = if (group == "exited" and queued) "running" else b, queued' = if (group == "exited") false else queued, group' = group, task' = task, strict' = strict }
  action startB = all { owner == "none", b == "pending", group == "exited", owner' = "b", b' = "running", queued' = false, group' = group, task' = task, pipe' = pipe, strict' = strict }
  action exitB = all { b == "running", b' = "exited", owner' = "none", group' = group, task' = task, pipe' = pipe, queued' = queued, strict' = strict }
  action step = any { rejectStrict, startA, borrowPipe, cancelA, queueB, exitGroup, closePipe, startB, exitB }
  action badExit = all { task == "cancelled", group == "running", pipe == "held", group' = "exited", owner' = "none", task' = task, pipe' = pipe, queued' = queued, b' = b, strict' = strict }
  action badStep = any { rejectStrict, startA, borrowPipe, cancelA, queueB, badExit, closePipe, startB, exitB }
  def safe = all { (group == "running" or pipe == "held") implies owner == "a", b == "running" implies owner == "b" }
  run heldAfterExitTest = init.then(rejectStrict).then(startA).then(borrowPipe).then(queueB).then(cancelA).then(exitGroup).then(closePipe).then(exitB)
  run closedBeforeExitTest = init.then(startA).then(borrowPipe).then(cancelA).then(queueB).then(closePipe).then(exitGroup).then(exitB)
}
