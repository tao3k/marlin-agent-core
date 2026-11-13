;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Shared admission bounds for Scheme UI objects and typed projections.
(import (only-in :clan/poo/object .slot? object?)
        (only-in :std/list/list length<=n?))
(export marlin-ui-max-contributions marlin-ui-max-extensions marlin-ui-max-content-items
        marlin-ui-max-browse-steps
        marlin-ui-bounded-label? marlin-ui-bounded-list? marlin-ui-required-object)

(def marlin-ui-max-contributions 8)
(def marlin-ui-max-extensions 16)
;;; Help derives rows from the finite global and mode-scoped keymap.
(def marlin-ui-max-content-items 64)
;;; At most sixteen coalesced pulses, each with a bounded step of 32.
(def marlin-ui-max-browse-steps 512)
(def max-label-length 256)

(def (marlin-ui-required-object value slots)
  (unless (and (object? value) (andmap (lambda (slot) (.slot? value slot)) slots))
    (error "invalid UI value")))

(def (marlin-ui-bounded-label? value)
  (and (string? value) (<= (string-length value) max-label-length)))

;;; Bound traversal first: list? then rejects an improper short tail.
(def (marlin-ui-bounded-list? values limit)
  (and (length<=n? values limit) (list? values)))
