// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// A Session journal, one Runtime task and one observing client. A commit and
// a client acknowledgement are separate actions. A dropped connection loses
// its delivery position and reconnects from the last acknowledged cursor.
module appServerTurn {
  type State = { phase: str, log: List[str], acknowledged: int, delivered: int,
    connected: bool, cancel: bool, localRetired: bool, receipt: str, dispatches: int }
  var s: State

  action init = s' = { phase: "idle", log: [], acknowledged: 0, delivered: 0,
    connected: true, cancel: false, localRetired: false, receipt: "none", dispatches: 0 }

  action commitStart = all {
    s.phase == "idle",
    s' = s.with("phase", "claimed").with("log", ["start"]),
  }
  action dispatch = all {
    s.phase == "claimed", not(s.cancel),
    s' = s.with("phase", "running").with("dispatches", s.dispatches + 1),
  }
  action beginCheckpoint = all {
    s.phase == "running", not(s.cancel), s.log.length() < 4,
    s' = s.with("phase", "writing-text"),
  }
  action commitCheckpoint = all {
    s.phase == "writing-text",
    s' = s.with("phase", "running").with("log", s.log.append("text")),
  }
  action beginFinish = all {
    s.phase == "running", not(s.cancel),
    s' = s.with("phase", "writing-finish"),
  }
  action commitFinish = all {
    s.phase == "writing-finish",
    s' = s.with("phase", "terminal").with("log", s.log.append("completed")),
  }
  action requestCancel = all {
    Set("claimed", "running", "writing-text", "writing-finish", "terminal").contains(s.phase),
    not(s.localRetired), not(s.cancel),
    s' = s.with("cancel", true),
  }
  action commitCancellation = all {
    Set("claimed", "running").contains(s.phase), s.cancel,
    s' = s.with("phase", "terminal").with("log", s.log.append("uncertain")),
  }
  action writeFailure = all {
    Set("writing-text", "writing-finish").contains(s.phase) or
      (Set("claimed", "running").contains(s.phase) and s.cancel),
    s' = s.with("phase", "write-error"),
  }
  action retire = all {
    Set("terminal", "write-error").contains(s.phase), not(s.localRetired),
    s' = s.with("localRetired", true),
  }
  action publishReceipt = all {
    s.phase == "terminal", s.localRetired, s.receipt != "joined",
    s' = s.with("receipt", "joined"),
  }
  action timeoutReceipt = all {
    s.cancel, not(s.localRetired), s.receipt == "none",
    s' = s.with("receipt", "timeout"),
  }
  action disconnect = all { s.connected, s' = s.with("connected", false) }
  action reconnect = all {
    not(s.connected),
    s' = s.with("connected", true).with("delivered", s.acknowledged),
  }
  action deliver = all {
    s.connected, s.delivered < s.log.length(),
    s' = s.with("delivered", s.delivered + 1),
  }
  action acknowledge = all {
    s.connected, s.acknowledged < s.delivered,
    s' = s.with("acknowledged", s.acknowledged + 1),
  }
  action step = any {
    commitStart, dispatch, beginCheckpoint, commitCheckpoint, beginFinish,
    commitFinish, requestCancel, commitCancellation, writeFailure, retire,
    publishReceipt, timeoutReceipt, disconnect, reconnect, deliver, acknowledge,
  }

  val hasTerminal = if (s.log.length() == 0) false else Set("completed", "uncertain").contains(s.log.nth(s.log.length() - 1))
  val safe = all {
    s.acknowledged >= 0, s.acknowledged <= s.delivered, s.delivered <= s.log.length(),
    s.dispatches <= 1,
    s.dispatches == 1 implies (s.log.length() > 0 and s.log.head() == "start"),
    s.receipt == "joined" implies (s.localRetired and hasTerminal),
    s.phase == "terminal" iff hasTerminal,
    (hasTerminal and s.log.nth(s.log.length() - 1) == "uncertain") implies s.cancel,
    s.phase == "write-error" implies not(hasTerminal),
  }

  run reconnectAndCancelTest = init.then(commitStart).then(dispatch).then(deliver)
    .then(acknowledge).then(disconnect).then(beginCheckpoint).then(commitCheckpoint)
    .then(reconnect).then(deliver).then(requestCancel).then(commitCancellation)
    .then(retire).then(publishReceipt).then(deliver)
    .expect(safe and s.delivered == 3 and s.log.nth(s.log.length() - 1) == "uncertain")
  run unacknowledgedDeliveryTest = init.then(commitStart).then(deliver)
    .then(disconnect).then(reconnect).expect(safe and s.delivered == 0)
  run finishCommitRacesCancelTest = init.then(commitStart).then(dispatch)
    .then(beginFinish).then(requestCancel).then(commitFinish).then(retire)
    .then(publishReceipt).expect(safe and s.log.nth(s.log.length() - 1) == "completed")
  run failedCommitTest = init.then(commitStart).then(dispatch).then(beginCheckpoint)
    .then(writeFailure).then(retire).expect(safe and s.log == ["start"] and s.receipt == "none")
  run timeoutDoesNotProveRetirementTest = init.then(commitStart).then(dispatch)
    .then(requestCancel).then(timeoutReceipt).expect(safe and not(s.localRetired))
}
