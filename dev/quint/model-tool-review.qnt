// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// One pending model-tool call. The runtime owns the handler and decision;
// the journal must commit the request before the UI may approve it.
module modelToolReview {
  var phase: str
  var registered: bool
  var durable: bool
  var turn: int
  var call: int
  var receiptTurn: int
  var receiptCall: int
  var claims: int
  var executions: int
  var catalogTool: int
  var requestedTool: int

  def safe = all {
    claims >= 0,
    claims <= 1,
    executions >= 0,
    executions <= claims,
    executions <= 1,
    executions > 0 implies
      (registered and durable and receiptTurn == turn and receiptCall == call),
    phase == "approved" implies
      (durable and receiptTurn == turn and receiptCall == call),
    (phase == "claimed" or phase == "uncertain") implies
      (durable and claims == 1 and receiptTurn == turn and receiptCall == call),
    (phase == "requested" or phase == "durable" or
      phase == "approved" or phase == "claimed" or
      phase == "uncertain" or phase == "executed") implies
      (registered and catalogTool == requestedTool),
    phase == "uncertain" implies executions == 0,
    phase == "rejected" implies executions == 0,
  }

  action init = all {
    phase' = "idle", registered' = false, durable' = false,
    turn' = 0, call' = 0, receiptTurn' = 0, receiptCall' = 0,
    claims' = 0, executions' = 0, catalogTool' = 0, requestedTool' = 0,
  }

  action register = all {
    phase == "idle",
    registered' = true, phase' = phase, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = 1, requestedTool' = requestedTool,
  }

  action offer = all {
    phase == "idle", registered,
    phase' = "offered", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = catalogTool,
  }

  action request = all {
    phase == "offered",
    phase' = "requested", registered' = registered, durable' = durable,
    turn' = 1, call' = 1, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action persist = all {
    phase == "requested",
    phase' = "durable", registered' = registered, durable' = true,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action approve = all {
    phase == "durable", registered,
    phase' = "approved", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = turn,
    receiptCall' = call, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action reject = all {
    phase == "durable",
    phase' = "rejected", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  // This journal write must finish before invoking a handler. A crash after
  // the claim has an unknown external outcome and cannot be retried silently.
  action claim = all {
    phase == "approved", registered, durable, claims == 0,
    receiptTurn == turn, receiptCall == call,
    phase' = "claimed", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = 1, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action execute = all {
    phase == "claimed", registered, durable, claims == 1,
    receiptTurn == turn, receiptCall == call, executions == 0,
    phase' = "executed", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = 1,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action crashAfterClaim = all {
    phase == "claimed", claims == 1, executions == 0,
    phase' = "uncertain", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action replay = all {
    phase == "executed" or phase == "uncertain",
    phase' = phase, registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action step = any {
    register, offer, request, persist, approve, reject,
    claim, execute, crashAfterClaim, replay
  }

  // Negative controls: neither a provisional request nor a stale receipt
  // can authorize execution for this turn and provider call ID.
  action badExecuteBeforeCommit = all {
    phase == "requested",
    phase' = "executed", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = turn,
    receiptCall' = call, claims' = claims, executions' = 1,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action badExecuteBeforeClaim = all {
    phase == "approved", claims == 0,
    phase' = "executed", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = 1,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action badApproveStaleCall = all {
    phase == "durable",
    phase' = "approved", registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = turn,
    receiptCall' = call + 1, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action badRequestUnknownTool = all {
    phase == "offered",
    phase' = "requested", registered' = registered, durable' = durable,
    turn' = 1, call' = 1, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = catalogTool + 1,
  }

  action badExecuteTwice = all {
    phase == "executed",
    phase' = phase, registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions + 1,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action badRetryAfterCrash = all {
    phase == "uncertain", claims == 1, executions == 0,
    phase' = phase, registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = 1,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  action badStep = any {
    step, badExecuteBeforeCommit, badExecuteBeforeClaim,
    badApproveStaleCall, badRequestUnknownTool, badExecuteTwice,
    badRetryAfterCrash
  }

  action observeUnsafe = all {
    assert(not(safe)),
    phase' = phase, registered' = registered, durable' = durable,
    turn' = turn, call' = call, receiptTurn' = receiptTurn,
    receiptCall' = receiptCall, claims' = claims, executions' = executions,
    catalogTool' = catalogTool, requestedTool' = requestedTool,
  }

  run approvedOnceTest = init.then(register).then(offer).then(request)
    .then(persist).then(approve).then(claim).then(execute).then(replay)

  run rejectedNeverExecutesTest = init.then(register).then(offer)
    .then(request).then(persist).then(reject)

  run earlyExecutionIsUnsafeTest = init.then(register).then(offer)
    .then(request).then(badExecuteBeforeCommit).then(observeUnsafe)

  run unclaimedExecutionIsUnsafeTest = init.then(register).then(offer)
    .then(request).then(persist).then(approve)
    .then(badExecuteBeforeClaim).then(observeUnsafe)

  run staleCallIsUnsafeTest = init.then(register).then(offer)
    .then(request).then(persist).then(badApproveStaleCall)
    .then(observeUnsafe)

  run unknownToolIsUnsafeTest = init.then(register).then(offer)
    .then(badRequestUnknownTool).then(observeUnsafe)

  run duplicateExecutionIsUnsafeTest = init.then(register).then(offer)
    .then(request).then(persist).then(approve).then(claim).then(execute)
    .then(badExecuteTwice).then(observeUnsafe)

  run claimedCrashCannotRetryTest = init.then(register).then(offer)
    .then(request).then(persist).then(approve).then(claim)
    .then(crashAfterClaim).then(replay)

  run retryAfterClaimedCrashIsUnsafeTest = init.then(register).then(offer)
    .then(request).then(persist).then(approve).then(claim)
    .then(crashAfterClaim).then(badRetryAfterCrash).then(observeUnsafe)
}
