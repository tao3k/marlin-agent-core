;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Link the C exports. The Rust owner opens the Deck on demand.
(import :marlin-agent-core/c-abi/deck-runtime)
(export main)

(def (main . _) (void))
