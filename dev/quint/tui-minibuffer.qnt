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

  pure val height = 30
  pure val topStatusRows = 0
  pure val modelineRows = 1
  pure val bottomHintRows = 1

  def composerRows = if (hovered or hasDraft) 3 else 0
  def modelineRow = height - bottomHintRows - composerRows - modelineRows
  def layoutSafe = topStatusRows == 0 and modelineRow >= 0 and
    modelineRow + modelineRows + composerRows + bottomHintRows == height
  def validFocus = focus == "composer" or focus == "commands" or focus == "transcript"
  def stateSafe = layoutSafe and validFocus and pointerRow >= 0 and pointerRow <= height

  action init = all {
    focus' = "composer",
    pointerRow' = 0,
    hovered' = false,
    hasDraft' = false,
    pointerEpoch' = 0,
    keyEpoch' = 0,
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
  }

  action typeDraft = all {
    hasDraft' = true,
    hovered' = hovered,
    pointerRow' = pointerRow,
    focus' = focus,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
  }

  action clearDraft = all {
    hasDraft' = false,
    hovered' = hovered,
    pointerRow' = pointerRow,
    focus' = focus,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
  }

  action nextFocus = all {
    focus' = if (focus == "composer") "commands" else if (focus == "commands") "transcript" else "composer",
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch + 1,
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
  }

  action observe(expectedHover, expectedRows, expectedKeys) = all {
    assert(hovered == expectedHover and composerRows == expectedRows and keyEpoch == expectedKeys),
    focus' = focus,
    pointerRow' = pointerRow,
    hovered' = hovered,
    hasDraft' = hasDraft,
    pointerEpoch' = pointerEpoch,
    keyEpoch' = keyEpoch,
  }

  run hoverRoundTripTest = init.then(movePointer(height)).then(observe(true, 3, 0))
    .then(movePointer(1)).then(observe(false, 0, 0))

  run draftKeepsEditorTest = init.then(typeDraft).then(movePointer(1))
    .then(observe(false, 3, 1))
}
