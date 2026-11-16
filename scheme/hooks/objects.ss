;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Assemble the operations of the Hooks module.

(import (only-in :clan/poo/object .o)
        (only-in "./selection.ss" select-hook))

(export HooksOperations)

(def HooksOperations
  (.o select: select-hook))
