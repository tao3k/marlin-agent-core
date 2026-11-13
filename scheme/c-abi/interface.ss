;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native adapters are a selected Module; Rust owns their ABI readiness.
(.def CAbi
  identity: 'c-abi
  core: (.o requires: (.o Context: (.o module-id: 'context)
                         Ui: (.o module-id: 'ui)
                         DeckRuntime: (.o module-id: 'deck-runtime)
                         Tooling: (.o module-id: 'tooling))
            scope-contract: CAbiScopeContract
            exports: (make-c-abi-scope Ui Context DeckRuntime Tooling)
            projections: (list Build))
  features: (.o))
