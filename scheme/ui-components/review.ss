;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Human review is a Minibuffer choice presentation. The runtime owns the
;;; gate and decision receipt; these entries carry no execution authority.
(import (only-in :clan/poo/object .def .get seal-object!))
(export marlin-ui-review-menu)

(.def UiReviewMenu.
  choices: '((approved "Approve once" "Continue with the approved graph")
             (rejected "Deny" "Stop this graph loop")
             (needs-question "Ask a question" "Keep this gate pending")
             (patch-requested "Request changes" "Keep this gate pending")))
(def marlin-ui-review-menu-config (seal-object! UiReviewMenu.))

(def (marlin-ui-review-menu review-kind)
  (let (prompt
        (case review-kind
          ((general) "Review: ")
          ((policy-change) "Policy: ")
          ((permission-escalation) "Permission: ")
          ((memory-promotion) "Memory: ")
          ((cross-project-reference) "Reference: ")
          ((session-visibility-expansion) "Visibility: ")
          ((data-migration) "Migration: ")
          (else (error "invalid human review kind"))))
    (vector prompt
            (list->vector
              (map (lambda (entry)
                     (vector (car entry) (cadr entry) (caddr entry)))
                   (.get marlin-ui-review-menu-config choices))))))
