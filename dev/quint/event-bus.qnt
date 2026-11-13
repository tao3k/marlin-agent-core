// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
module eventBus {
  var q1: int
  var q2: int
  var active1: bool
  var active2: bool
  action init = all { q1' = 0, q2' = 0, active1' = true, active2' = true }
  action publish = all { (not(active1) or q1 == 0), (not(active2) or q2 == 0), active1 or active2, q1' = if (active1) 1 else q1, q2' = if (active2) 1 else q2, active1' = active1, active2' = active2 }
  action take1 = all { q1 == 1, q1' = 0, q2' = q2, active1' = active1, active2' = active2 }
  action take2 = all { q2 == 1, q2' = 0, q1' = q1, active1' = active1, active2' = active2 }
  action close1 = all { active1, active1' = false, active2' = active2, q1' = 0, q2' = q2 }
  action close2 = all { active2, active2' = false, active1' = active1, q2' = 0, q1' = q1 }
  action step = any { publish, take1, take2, close1, close2 }
  action badPublish = all { active1, q1 == 1, q1' = q1 + 1, q2' = q2, active1' = active1, active2' = active2 }
  action badStep = any { step, badPublish }
  def safe = all { q1 >= 0, q1 <= 1, q2 >= 0, q2 <= 1, not(active1) implies q1 == 0, not(active2) implies q2 == 0 }
  run fanoutReleaseTest = init.then(publish).then(take1).then(take2).then(publish).then(close1).then(close2)
}
