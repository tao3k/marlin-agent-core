;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Shared admission bounds for Scheme UI objects and typed projections.
(import (only-in :clan/poo/object .slot? object?))
(export marlin-ui-max-contributions marlin-ui-max-extensions marlin-ui-max-content-items
        marlin-ui-bounded-label? marlin-ui-bounded-list? marlin-ui-required-object)

(def marlin-ui-max-contributions 8)
(def marlin-ui-max-extensions 16)
(def marlin-ui-max-content-items 16)
(def max-label-length 256)

(def (marlin-ui-required-object value slots)
  (unless (and (object? value) (andmap (lambda (slot) (.slot? value slot)) slots))
    (error "invalid UI value")))

(def (marlin-ui-bounded-label? value)
  (and (string? value) (<= (string-length value) max-label-length)))

;;; Reject improper and cyclic lists before any unbounded traversal.
(def (marlin-ui-bounded-list? values limit)
  (let loop ((rest values) (remaining limit))
    (cond ((null? rest) #t)
          ((or (zero? remaining) (not (pair? rest))) #f)
          (else (loop (cdr rest) (- remaining 1))))))
