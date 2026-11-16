// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Finite UI control model. The Rust renderer owns geometry and pixels;
// Scheme owns the selected modeline slots and focus dispatch.
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
