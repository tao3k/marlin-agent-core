;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Deck owns execution of typed Context batches on Scheme actors.

(.def DeckRuntime
  identity: 'deck-runtime
  core: (.o scope-contract: DeckScopeContract
            exports: DeckScope)
  features: (.o))
