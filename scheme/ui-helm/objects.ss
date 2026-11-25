;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO prototypes for Marlin's Modern Helm (`scheme/ui-helm`).
;;; Decouples Sources, Two-Stage Query (Async Search Playbook + Local Orderless),
;;; Marginalia Annotations, Virtual Viewport, Preview, and Embark Actions.
(import (only-in :clan/poo/object .def))
(export HelmSource.
        HelmParsedInput.
        HelmCandidate.
        HelmAction.
        HelmActionReceipt.
        HelmIndexPolicy.
        HelmWindowPolicy.
        HelmSession.)

(.def HelmIndexPolicy.
  skip-dirs: '("target" ".git" ".gerbil" "node_modules" ".devenv" ".direnv" "dist" "build")
  max-file-bytes: 2097152
  max-line-matches: 6
  max-line-preview-chars: 220
  max-hits: 64
  context-radius: 5
  fuzzy-distance: 1
  enable-tantivy?: #t
  enable-file-locator?: #t
  enable-rg-regex?: #t)

(.def HelmWindowPolicy.
  window-style: 'bottom-split
  height-percent: 54
  preview-width-percent: 44
  preview-min-width: 88
  source-headers?: #t
  match-highlight?: #t
  follow-preview?: #t)

(.def HelmSource.
  id: 'search
  label: "Search"
  kind: 'search
  priority: 10
  async?: #t)

(.def HelmParsedInput.
  raw-text: ""
  async-pattern: ""
  local-tokens: '()
  languages: '()
  documents: '()
  path-prefix: #f
  extension: #f)

(.def HelmCandidate.
  id: ""
  source-id: 'search
  display: ""
  path: #f
  line: #f
  language: #f
  axes: '()
  score: 0
  annotation: ""
  preview-lines: '())

(.def HelmAction.
  id: 'open
  key: "RET"
  label: "Open"
  scope: 'single)

(.def HelmActionReceipt.
  action-id: 'open
  epoch: 0
  generation-digest: #f
  target-ids: '()
  status: 'accepted)

(.def HelmSession.
  open?: #f
  workspace: "marlin-main"
  input-text: ""
  parsed: HelmParsedInput.
  epoch: 0
  sources: '()
  active-source: 'all
  raw-candidates: '()
  candidates: '()
  selected-index: 0
  marked-ids: '()
  viewport-offset: 0
  viewport-height: 10
  layout: 'split-preview
  action-menu-open?: #f
  generation-digest: #f
  last-action-receipt: #f)
