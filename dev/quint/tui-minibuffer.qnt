// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Finite UI control model. Scheme owns layout policy and focus dispatch;
// the Rust renderer measures and clips terminal cells.
module tuiMinibuffer {
  var focus: str
  var pointerRow: int
  var hovered: bool
  var hasDraft: bool
  var pointerEpoch: int
  var keyEpoch: int
  var commandOpen: bool
  var selectedCommand: int
  var askOpen: bool

  pure val height = 30
  pure val topStatusRows = 0
  pure val modelineRows = 1
  // Small finite abstraction: these are not production catalog or layout limits.
  pure val commandCount = 2
  pure val visibleCommandRows = 1

  def bottomHintRows = if (commandOpen or askOpen) 0 else 1
  def composerRows = if (commandOpen or askOpen) 1 else if (hovered or hasDraft) 3 else 0
  def commandRows = if (commandOpen) visibleCommandRows else 0
  def askRows = if (askOpen) 4 else 0
  def modelineRow = height - bottomHintRows - commandRows - askRows - composerRows - modelineRows
  def layoutSafe = topStatusRows == 0 and modelineRow >= 0 and
    modelineRow + modelineRows + composerRows + commandRows + askRows + bottomHintRows == height and
    not(commandOpen and askOpen) and
    (not(commandOpen) or (bottomHintRows == 0 and
      modelineRow + modelineRows < height - commandRows)) and
    (not(askOpen) or (bottomHintRows == 0 and
      modelineRow + modelineRows < height - askRows))
  def validFocus = focus == "composer" or focus == "commands" or focus == "transcript"
  def commandSafe = if (commandOpen) selectedCommand >= 0 and
    selectedCommand < commandCount else selectedCommand == 0
  def stateSafe = layoutSafe and validFocus and commandSafe and
    pointerRow >= 0 and pointerRow <= height

  action init = all {
    focus' = "composer",
    pointerRow' = 0,
    hovered' = false,
    hasDraft' = false,
    pointerEpoch' = 0,
    keyEpoch' = 0,
    commandOpen' = false,
    selectedCommand' = 0,
    askOpen' = false,
  }

  // Pointer motions overwrite the sampled position. They never enqueue keys.
  action movePointer(row) = all {
    row >= 0 and row <= height,
    pointerRow' = row,
    hovered' = row > height - (if (hovered or hasDraft) 3 else 0) - 1,
    hasDraft' = hasDraft,
    focus' = focus,
    pointerEpoch' = pointerEpoch + 1,
    keyEpoch' = keyEpoch,
    commandOpen' = commandOpen,
    selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  action typeDraft = all {
    not(askOpen),
    hasDraft' = true,
    hovered' = hovered,
    pointerRow' = pointerRow,
    focus' = focus,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
    commandOpen' = commandOpen,
    selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  action clearDraft = all {
    not(askOpen),
    hasDraft' = false,
    hovered' = hovered,
    pointerRow' = pointerRow,
    focus' = focus,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
    commandOpen' = commandOpen,
    selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  action nextFocus = all {
    not(askOpen),
    focus' = if (focus == "composer") "commands" else if (focus == "commands") "transcript" else "composer",
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
    commandOpen' = commandOpen,
    selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  action openCommands = all {
    not(commandOpen), not(askOpen),
    focus' = "commands",
    commandOpen' = true,
    selectedCommand' = 0,
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
    askOpen' = askOpen,
  }

  // The first arrow changes selection in the same input transition. A draw
  // is not a prerequisite for input admission.
  action moveSelection = all {
    commandOpen,
    focus' = focus,
    commandOpen' = commandOpen,
    selectedCommand' = (selectedCommand + 1) % commandCount,
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
    askOpen' = askOpen,
  }

  action closeCommands = all {
    commandOpen,
    focus' = "composer",
    commandOpen' = false,
    selectedCommand' = 0,
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
    askOpen' = askOpen,
  }

  // Ask temporarily occupies the same Mini Buffer; the chat draft and
  // selected window remain intact while Scheme supplies its choice frame.
  action openAsk = all {
    not(commandOpen), not(askOpen),
    focus' = focus, commandOpen' = commandOpen,
    selectedCommand' = selectedCommand, askOpen' = true,
    pointerRow' = pointerRow, hovered' = hovered,
    hasDraft' = hasDraft, pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch,
  }

  action closeAsk = all {
    askOpen,
    focus' = focus, commandOpen' = commandOpen,
    selectedCommand' = selectedCommand, askOpen' = false,
    pointerRow' = pointerRow, hovered' = hovered,
    hasDraft' = hasDraft, pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch,
  }

  action step = any {
    movePointer(0),
    movePointer(1),
    movePointer(height - 4),
    movePointer(height - 3),
    movePointer(height),
    typeDraft,
    clearDraft,
    nextFocus,
    openCommands,
    moveSelection,
    closeCommands,
    openAsk,
    closeAsk,
  }

  action observe(expectedHover, expectedRows, expectedKeys) = all {
    assert(hovered == expectedHover and composerRows == expectedRows and keyEpoch == expectedKeys),
    focus' = focus,
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch,
    commandOpen' = commandOpen,
    selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  action observeCommands(expectedSelection, expectedKeys) = all {
    assert(commandOpen and selectedCommand == expectedSelection and
      bottomHintRows == 0 and modelineRow + modelineRows < height - commandRows and
      keyEpoch == expectedKeys),
    focus' = focus,
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch,
    commandOpen' = commandOpen,
    selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  action observeAsk(expectedDraft, expectedKeys) = all {
    assert(askOpen and not(commandOpen) and hasDraft == expectedDraft and
      bottomHintRows == 0 and modelineRow + modelineRows < height - askRows and
      keyEpoch == expectedKeys),
    focus' = focus, pointerRow' = pointerRow,
    hovered' = hovered, hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch, keyEpoch' = keyEpoch,
    commandOpen' = commandOpen, selectedCommand' = selectedCommand,
    askOpen' = askOpen,
  }

  run hoverRoundTripTest = init.then(movePointer(height)).then(observe(true, 3, 0))
    .then(movePointer(1)).then(observe(false, 0, 0))

  run draftKeepsEditorTest = init.then(typeDraft).then(movePointer(1))
    .then(observe(false, 3, 1))

  run firstArrowAfterMxTest = init.then(openCommands)
    .then(observeCommands(0, 1)).then(moveSelection)
    .then(observeCommands(1, 2)).then(closeCommands)
    .then(observe(false, 0, 3))

  run askRetainsDraftTest = init.then(typeDraft).then(openAsk)
    .then(observeAsk(true, 1)).then(closeAsk)
    .then(observe(false, 3, 1))
}

// MAR-UI-R25. Echo's timed transitions leave the editing and log owners intact.
module minibufferEcho {
  type State = {
    now: int, echo: str, keyUntil: int, keypad: str, paintedAt: int,
    notice: bool, noticeAt: int, messages: int, draft: str, query: str,
  }
  var s: State

  pure val hintDelay = 500
  pure val echoLifetime = 3000
  def hasPainted = s.paintedAt >= 0
  def safe = all {
    s.keypad == "visible" implies
      (s.echo == "pending" and hasPainted and s.now >= s.paintedAt + hintDelay),
    s.keypad != "closed" implies s.echo == "pending",
    s.echo == "completed" implies s.now < s.keyUntil,
    s.notice implies (s.messages > 0 and s.now < s.noticeAt + echoLifetime),
    s.messages >= 0 and s.messages <= 64,
    s.draft == "draft" and s.query == "query",
  }

  action init = s' = {
    now: 0, echo: "none", keyUntil: 0, keypad: "closed", paintedAt: -1,
    notice: false, noticeAt: -1, messages: 0, draft: "draft", query: "query",
  }
  action beginPrefix = all {
    s.echo != "pending",
    s' = s.with("echo", "pending").with("notice", false)
      .with("keypad", "queued").with("paintedAt", -1),
  }
  action paint = all {
    s.keypad == "queued", not(hasPainted),
    s' = s.with("paintedAt", s.now),
  }
  action finishPrefix = all {
    s.echo == "pending",
    s' = s.with("echo", "completed").with("keypad", "closed")
      .with("keyUntil", s.now + echoLifetime).with("notice", false),
  }
  action dismiss = s' = s.with("echo", "none").with("keypad", "closed").with("notice", false)
  action publish = s' = s.with("notice", true).with("noticeAt", s.now)
    .with("messages", if (s.messages < 64) s.messages + 1 else 64)
  action tick(elapsed) = {
    val now = s.now + elapsed
    s' = s.with("now", now)
      .with("notice", s.notice and now < s.noticeAt + echoLifetime)
      .with("echo", if (s.echo == "completed" and now >= s.keyUntil) "none" else s.echo)
      .with("keypad", if (s.keypad == "queued" and hasPainted and now >= s.paintedAt + hintDelay)
        "visible" else s.keypad)
  }
  action step = any {
    beginPrefix, paint, finishPrefix, dismiss, publish,
    tick(hintDelay - 1), tick(1), tick(echoLifetime),
  }

  // A timer started at prefix admission could reveal help before the first frame.
  action earlyHint = all {
    s.keypad == "queued", not(hasPainted),
    s' = s.with("keypad", "visible"),
  }
  action badStep = any { beginPrefix, earlyHint }

  action observe(predicate) = all { assert(predicate), s' = s }
  run hintStartsAfterPaintTest = init.then(beginPrefix).then(tick(echoLifetime))
    .then(observe(s.keypad == "queued" and s.echo == "pending"))
    .then(paint).then(tick(hintDelay - 1)).then(observe(s.keypad == "queued"))
    .then(tick(1)).then(observe(s.keypad == "visible" and safe))
  run cancelledHintStaysClosedTest = init.then(beginPrefix).then(paint).then(dismiss)
    .then(tick(hintDelay)).then(observe(s.keypad == "closed" and safe))
  run replacementSurvivesOldDeadlineTest = init.then(publish).then(tick(hintDelay)).then(publish)
    .then(tick(echoLifetime - hintDelay)).then(observe(s.notice and s.messages == 2 and safe))
    .then(tick(hintDelay)).then(observe(not(s.notice) and s.messages == 2 and safe))
  run dismissPreservesLogTest = init.then(beginPrefix).then(finishPrefix).then(publish).then(dismiss)
    .then(observe(s.echo == "none" and not(s.notice) and s.messages == 1 and safe))
}
