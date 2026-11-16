;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validate the Deck publication before exposing its operations.

(import (only-in :clan/poo/mop validate)
        "./objects.ss"
        "./types.ss")

(export DeckScope)

(def DeckScope (validate DeckScopeContract DeckOperations))
