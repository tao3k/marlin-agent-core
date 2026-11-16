;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The selected native adapter Module owns its compiler inputs.
(def Build
  (append
    (cond-expand
      (darwin '((gxc: "roots" "-ld-options" "-Wl,-undefined,dynamic_lookup")))
      (else '("roots")))
    '("context" "deck-runtime" "ui" "tooling" "context-admission")))
