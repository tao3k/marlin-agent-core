;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Direct admission uses the same selected Context bridge as Deck actors.
(import (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
(export marlin-context-admit-batch)
(def (marlin-context-admit-batch input)
  (def context (.get marlin-agent-core.init.exports Marlin context))
  ((.get context bridge encode-scopes)
    (vector-map (.get context admit-scope) ((.get context bridge decode-batch) input))))
