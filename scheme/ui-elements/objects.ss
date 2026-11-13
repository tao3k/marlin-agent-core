;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native Scheme vectors carry the generic UI vocabulary across the ABI.
(export ui-text ui-rows ui-columns ui-region ui-template ui-choice ui-sized ui-input ui-slot ui-panel
        ui-size-cells ui-size-flex ui-size-binding ui-size-bounded ui-size-choice ui-sized-with
        ui-size-min ui-size-max ui-size-add ui-size-subtract)

(def (ui-text spans-value)
  (vector 'text (list->vector (map (lambda (span) (vector (car span) (cdr span))) spans-value))))
(def (ui-rows children-value) (vector 'rows (list->vector children-value)))
(def (ui-columns children-value) (vector 'columns (list->vector children-value)))
(def (ui-region identity-value scroll-target-value child-value)
  (vector 'region (vector identity-value scroll-target-value child-value)))
(def (ui-sized kind-value size-value child-value)
  (vector 'sized (vector kind-value size-value child-value)))
(def (ui-size-cells count-value) (vector 'cells count-value))
(def (ui-size-flex weight-value) (vector 'flex weight-value))
(def (ui-size-binding identity-value) (vector 'binding identity-value))
(def (ui-size-bounded value-identity limit-identity)
  (vector 'bounded (vector value-identity limit-identity)))
(def (ui-size-min left right) (vector 'min (vector left right)))
(def (ui-size-max left right) (vector 'max (vector left right)))
(def (ui-size-add left right) (vector 'add (vector left right)))
(def (ui-size-subtract left right) (vector 'subtract (vector left right)))
(def (ui-size-choice cases-value fallback-value)
  (vector 'choice (vector (list->vector (map list->vector cases-value)) fallback-value)))
(def (ui-sized-with size-value child-value)
  (ui-sized (vector-ref size-value 0) (vector-ref size-value 1) child-value))
(def (ui-input bordered-value background-value prefix-value title-value active-value)
  (vector 'input (vector bordered-value background-value prefix-value title-value active-value)))
(def (ui-slot identity-value) (vector 'slot (vector identity-value)))
(def (ui-panel border-tone-value field-tone-value child-value)
  (vector 'panel (vector border-tone-value field-tone-value child-value)))
(def (ui-template background-value spans-value)
  (vector 'template (vector background-value (list->vector (map list->vector spans-value)))))
(def (ui-choice cases-value fallback-value)
  (vector 'choice (vector (list->vector (map list->vector cases-value)) fallback-value)))
