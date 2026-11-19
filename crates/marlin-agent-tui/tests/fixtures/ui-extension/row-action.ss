;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Compiled separately from the package's declarations and keymap.
(import (only-in :clan/poo/object .o))
(export select-package-row)

(def (append-receipt message)
  (let (receipt (getenv "MARLIN_TEST_UI_AUTOLOAD_RECEIPT" #f))
    (when receipt
      (call-with-output-file (list path: receipt append: #t)
        (lambda (port) (display message port))))))

(append-receipt "ROW-ACTION-LOADED\n")

(def (select-package-row row)
  (append-receipt "ROW-ACTION-CALLED\n")
  (.o kind: 'focus identity: (if (eq? row 'first) 'commands 'composer)))
