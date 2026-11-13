// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// A Session journal, one Runtime task and one observing client. A commit and
// a client acknowledgement are separate actions. A dropped connection loses
// its delivery position and reconnects from the last acknowledged cursor.
// A process restart also loses the live owner of an unfinished turn.
module appServerTurn {
  type State = { phase: str, log: List[str], acknowledged: int, delivered: int,
    connected: bool, cancel: bool, ownerPresent: bool, localRetired: bool, receipt: str, dispatches: int,
    submission: str, startResponse: str, cancellationResponse: str }
  var s: State

  action init = s' = { phase: "idle", log: [], acknowledged: 0, delivered: 0,
    connected: true, cancel: false, ownerPresent: false, localRetired: false, receipt: "none", dispatches: 0,
    submission: "none", startResponse: "none", cancellationResponse: "unread" }

  action commitStart = {
    nondet id = Set("client-a", "client-b").oneOf()
    all {
      s.phase == "idle",
      s' = s.with("phase", "claimed").with("log", ["start"]).with("submission", id)
        .with("ownerPresent", true),
    }
  }
  action retryStart(id: str, sameInput: bool): bool = all {
    s.connected, s.submission != "none",
    s' = s.with("startResponse", if (id != s.submission) "busy"
      else if (sameInput) s.submission else "conflict"),
  }
  action retryAnyStart = {
    nondet id = Set("client-a", "client-b").oneOf()
    nondet sameInput = Set(true, false).oneOf()
    retryStart(id, sameInput)
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
    s.ownerPresent, not(s.localRetired), not(s.cancel),
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
    Set("terminal", "write-error").contains(s.phase), s.ownerPresent, not(s.localRetired),
    s' = s.with("localRetired", true),
  }
  action publishReceipt = all {
    s.phase == "terminal", s.localRetired, s.receipt != "joined",
    s' = s.with("receipt", "joined"),
  }
  action timeoutReceipt = all {
    s.cancel, s.ownerPresent, not(s.localRetired), s.receipt == "none",
    s' = s.with("receipt", "timeout"),
  }
  action restartWithUnfinishedTurn = all {
    Set("claimed", "running", "writing-text", "writing-finish", "write-error").contains(s.phase),
    s' = s.with("phase", "owner-lost").with("connected", false)
      .with("delivered", s.acknowledged).with("cancel", false)
      .with("ownerPresent", false).with("localRetired", false).with("cancellationResponse", "unread"),
  }
  action restartAfterTerminal = all {
    s.phase == "terminal", s.connected, s.ownerPresent,
    s' = s.with("connected", false).with("delivered", s.acknowledged)
      .with("ownerPresent", false).with("localRetired", false).with("cancellationResponse", "unread"),
  }
  action readCancellation = all {
    s.connected, s.submission != "none",
    s' = s.with("cancellationResponse", if (s.receipt == "none") "unavailable" else s.receipt),
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
    publishReceipt, timeoutReceipt, restartWithUnfinishedTurn, restartAfterTerminal,
    readCancellation, disconnect, reconnect, deliver, acknowledge, retryAnyStart,
  }

  val hasTerminal = if (s.log.length() == 0) false else Set("completed", "uncertain").contains(s.log.nth(s.log.length() - 1))
  val safe = all {
    s.acknowledged >= 0, s.acknowledged <= s.delivered, s.delivered <= s.log.length(),
    s.dispatches <= 1,
    (s.submission != "none") iff s.log.length() > 0,
    Set("client-a", "client-b").contains(s.startResponse) implies s.startResponse == s.submission,
    s.dispatches == 1 implies (s.log.length() > 0 and s.log.head() == "start"),
    s.receipt == "joined" implies hasTerminal,
    s.localRetired implies s.ownerPresent,
    s.cancellationResponse == "joined" implies s.receipt == "joined",
    s.phase == "terminal" iff hasTerminal,
    (hasTerminal and s.log.nth(s.log.length() - 1) == "uncertain") implies s.cancel,
    s.phase == "write-error" implies not(hasTerminal),
    s.phase == "owner-lost" implies (not(s.localRetired) and s.receipt != "joined"),
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
    .then(requestCancel).then(timeoutReceipt).then(restartWithUnfinishedTurn)
    .then(reconnect).then(readCancellation)
    .expect(safe and not(s.localRetired) and s.cancellationResponse == "timeout")
  run cancellationEvidenceSurvivesRestartTest = init.then(commitStart).then(dispatch)
    .then(requestCancel).then(commitCancellation).then(retire).then(publishReceipt)
    .then(restartAfterTerminal).then(reconnect).then(readCancellation)
    .expect(safe and s.cancellationResponse == "joined" and s.dispatches == 1
      and s.log == ["start", "uncertain"])
  run lostStartResponseTest = init.then(commitStart).then(disconnect).then(dispatch)
    .then(reconnect).then(retryStart(s.submission, true))
    .expect(safe and s.startResponse == s.submission and s.dispatches == 1)
  run restartAfterLostStartResponseTest = init.then(commitStart).then(disconnect)
    .then(dispatch).then(beginCheckpoint).then(commitCheckpoint)
    .then(restartWithUnfinishedTurn).then(reconnect)
    .then(retryStart(s.submission, true))
    .expect(safe and s.phase == "owner-lost" and s.startResponse == s.submission)
    .then(retryStart(s.submission, false)).then(deliver).then(deliver).then(readCancellation)
    .expect(safe and s.startResponse == "conflict" and s.dispatches == 1
      and s.log == ["start", "text"] and s.delivered == 2
      and s.cancellationResponse == "unavailable")
  run conflictingRetryTest = init.then(commitStart).then(dispatch)
    .then(retryStart(s.submission, false))
    .expect(safe and s.startResponse == "conflict" and s.dispatches == 1)
  run competingSubmissionTest = init.then(commitStart).then(dispatch)
    .then(retryStart(if (s.submission == "client-a") "client-b" else "client-a", true))
    .expect(safe and s.startResponse == "busy" and s.dispatches == 1)
}
