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
        HelmSearchPresets.
        HelmIndexPolicy.
        HelmWindowPolicy.
        HelmSession.)

;;; Declarative Helm mode presets owned by Scheme (projected to TUI over C-ABI).
;;; Uses language-agnostic General Syntax Ontology & Dusa finite-choice forms.
(.def HelmSearchPresets.
  entries:
  (list
    #("1" "search" "1:Search" "1/5 L0: Brute-Force Trigram/BM25"
      "fn ")
    #("2" "ascent" "2:Ascent" "2/5 Case A: Polyglot Ontology Callables"
      "(ascent-fragment ((answer lang path name line) <-- (file f lang path) (callable_def _ f lang name line)))")
    #("3" "imports" "3:Imports" "3/5 Case A: Imports Graph"
      "(ascent-fragment ((answer src dst path line) <-- (imports_edge src dst path line)))")
    #("4" "dusa" "4:Dusa-Resolve" "4/5 Case B.1: Dusa Call-Site Disambiguation"
      "(search-choice-program (scope limit: 12) (choice resolve (callsite target)) (rule (closed (resolve ?call) (?def)) (ref-at ?call ?name) (same-module-def ?call ?name ?def)) (rule (open (resolve ?call) ?def) (ref-at ?call ?name) (workspace-def ?name ?def)) (rule (closed (resolve ?call) ()) (resolve ?call ?def) (in-test-file ?def)) (solve resolve state-budget: 64 provenance?: #t))")
    #("5" "tier" "5:Dusa-Tier" "5/5 Case B.2: Dusa Architecture Tier"
      "(search-choice-program (scope limit: 4) (choice tier (mod level)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m 'provider)) (solve tier state-budget: 64 provenance?: #t))")))

(.def HelmIndexPolicy.
  skip-dirs: '("target" ".git" ".gerbil" ".data" "node_modules" ".devenv" ".direnv" "dist" "build")
  max-file-bytes: 2097152
  max-line-matches: 6
  max-line-preview-chars: 220
  max-hits: 64
  context-radius: 5
  fuzzy-distance: 1
  enable-tantivy?: #t
  enable-file-locator?: #t
  enable-rg-regex?: #t
  asp-inspect-batch-files: 512
  asp-hydration-max-files: 2048
  asp-hydration-chunk-size: 24
  search-presets: HelmSearchPresets.)

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
  extension: #f
  symbol: #f
  graph-target: #f)

(.def HelmCandidate.
  id: ""
  source-id: 'search
  display: ""
  path: #f
  line: #f
  language: #f
  primary-symbol: #f
  graph-relations: '()
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
