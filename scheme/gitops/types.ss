;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/type Type. define-type))
(export WorkingCopyObservation)

(define-type (WorkingCopyObservation @ Type.)
  .element?: (lambda (request)
               (and (vector? request) (= 3 (vector-length request))
                    (boolean? (vector-ref request 0))
                    (boolean? (vector-ref request 1))
                    (boolean? (vector-ref request 2)))))
