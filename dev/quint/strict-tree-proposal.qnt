// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Proposed Linux strict-containment protocol; not connected to production Rust.
// The held child runs only trusted setup until placement and restriction finish.
module strictTreeProposal {
  var group: str
  var child: str
  var authority: str
  var pipe: str
  var cancelled: bool
  var quota: str

  action init = all { group' = "none", child' = "none", authority' = "unrestricted", pipe' = "open", cancelled' = false, quota' = "held" }
  action create = all { group == "none", group' = "created", child' = child, authority' = authority, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action forkHeld = all { group == "created", child == "none", not(cancelled), child' = "held", group' = group, authority' = authority, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action attach = all { child == "held", group == "created", not(cancelled), group' = "populated", child' = child, authority' = authority, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action restrict = all { child == "held", group == "populated", authority == "unrestricted", not(cancelled), authority' = "restricted", group' = group, child' = child, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action execute = all { child == "held", group == "populated", authority == "restricted", not(cancelled), child' = "running", group' = group, authority' = authority, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action cancel = all { not(cancelled), cancelled' = true, group' = group, child' = child, authority' = authority, pipe' = pipe, quota' = quota }
  action cleanup = all { cancelled, group != "empty", group' = "empty", child' = "dead", authority' = authority, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action closePipe = all { pipe == "open", pipe' = "closed", group' = group, child' = child, authority' = authority, cancelled' = cancelled, quota' = quota }
  action release = all { cancelled, group == "empty", pipe == "closed", quota == "held", quota' = "free", group' = group, child' = child, authority' = authority, pipe' = pipe, cancelled' = cancelled }
  action step = any { create, forkHeld, attach, restrict, execute, cancel, cleanup, closePipe, release }
  action badExecute = all { child == "held", group == "created", not(cancelled), child' = "running", group' = group, authority' = authority, pipe' = pipe, cancelled' = cancelled, quota' = quota }
  action badRelease = all { cancelled, group == "empty", pipe == "open", quota == "held", quota' = "free", group' = group, child' = child, authority' = authority, pipe' = pipe, cancelled' = cancelled }
  action badLaunchStep = any { create, forkHeld, attach, restrict, execute, cancel, cleanup, closePipe, release, badExecute }
  action badReleaseStep = any { create, forkHeld, attach, restrict, execute, cancel, cleanup, closePipe, release, badRelease }
  def safe = all { child == "running" implies (group == "populated" and authority == "restricted"), quota == "free" implies (group == "empty" and pipe == "closed") }
  run cancelledAfterExecutionTest = init.then(create).then(forkHeld).then(attach).then(restrict).then(execute).then(cancel).then(cleanup).then(closePipe).then(release)
  run cancelledBeforePlacementTest = init.then(create).then(forkHeld).then(cancel).then(cleanup).then(closePipe).then(release)
}
