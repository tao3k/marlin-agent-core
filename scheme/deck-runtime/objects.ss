;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Deck publishes runtime metadata and opens actors on demand.

(import (only-in :clan/poo/object .o)
        (only-in :marlin-agent-core/deck-runtime/runtime open-deck-runtime))

(export DeckOperations)

(def DeckOperations
  (.o metadata: (.o module-id: "deck-runtime" version: 5)
      open: open-deck-runtime))
