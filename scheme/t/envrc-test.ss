;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in "../envrc/config.ss" EnvrcScope))

(def plan (.get EnvrcScope plan))

(for-each
  (lambda (example)
    (let ((operation (car example))
          (expected (cadr example)))
      (unless (and (eq? (plan (.o intent: operation project-envrc?: #t)) expected)
                   (eq? (plan (.o intent: operation project-envrc?: #f)) 'absent))
        (error "unexpected workspace ENVRC plan" operation))))
  '((activate export)
    (allow allow-and-export)
    (reload reload-and-export)
    (deny deny)))

(unless (with-catch (lambda (_) #t)
          (lambda () (plan (.o intent: 'authorize project-envrc?: #t)) #f))
  (error "unknown ENVRC intent was accepted"))

(displayln "ENVRC-POLICY-OK")
