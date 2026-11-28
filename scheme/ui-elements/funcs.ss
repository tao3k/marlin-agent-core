;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native Scheme vectors carry the generic UI vocabulary across the ABI.
(import (only-in :std/list/list group-n-consecutive))
(export ui-binding-present ui-text ui-rows ui-columns ui-region ui-scroll-target
        ui-template ui-prompt ui-choice ui-sized ui-input ui-slot ui-panel ui-list ui-bar
        ui-size-cells ui-size-flex ui-size-binding ui-size-bounded ui-size-choice ui-sized-with
        ui-size-min ui-size-max ui-size-add ui-size-subtract ui-key-hint-table)

;;; Each group supplies a heading and (key label tone) entries. Fill the
;;; available width, with smaller margins and gutters on narrow screens.
(def (ui-key-hint-table groups)
  (def (column heading entries)
    (def key-width (apply max (map (lambda (entry) (string-length (car entry))) entries)))
    (def width
      (apply max (string-length heading)
        (map (lambda (entry) (+ key-width 3 (string-length (cadr entry)))) entries)))
    (list width
      (ui-text
        (cons* (cons 'info heading)
               (cons 'muted (string-append "\n" (make-string width #\─)))
               (foldr (lambda (entry tail)
                        (let ((key (car entry)) (label (cadr entry)) (tone (caddr entry)))
                          (cons* (cons 'muted "\n")
                                 (cons 'accent key)
                                 (cons 'muted (string-append (make-string (- key-width (string-length key)) #\space)
                                                             " : "))
                                 (cons tone label)
                                 tail)))
                      '() entries)))))
  ;; Six heads plus the label and rule fit the existing eight-row bound.
  (def columns
    (apply append
      (map (lambda (group) (map (cut column (car group) <>)
                               (group-n-consecutive 6 (cadr group))))
           groups)))
  (def empty (ui-text '()))
  (def (spacer width) (ui-sized-with (ui-size-cells width) empty))
  (def (strip margin gap)
    (ui-columns
      (cons (spacer margin)
        (append (foldr (lambda (entry tail)
                         (cons (ui-sized-with (ui-size-flex 1) (cadr entry))
                               (if (null? tail) tail (cons (spacer gap) tail))))
                       '() columns)
                (list (spacer margin))))))
  (if (null? columns) empty
    (let* ((count (length columns))
           (width (+ 8 (* 4 (- count 1)) (* count (apply max (map car columns)))))
           (wide (string->symbol (string-append "width-at-least-" (number->string width)))))
      (ui-choice (list (list wide (strip 4 4)))
                 (strip 1 2)))))

(def (ui-text spans-value)
  (vector 'text (list->vector (map (lambda (span) (vector (car span) (cdr span))) spans-value))))
(def (ui-rows children-value) (vector 'rows (list->vector children-value)))
(def (ui-columns children-value) (vector 'columns (list->vector children-value)))
(def (ui-region identity-value scroll-target-value child-value)
  (vector 'region (vector identity-value scroll-target-value child-value)))
(def (ui-scroll-target identity-value step-value) (vector identity-value step-value))
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
(def (ui-list identity-value source-value offset-value row-value)
  (vector 'list (vector identity-value source-value offset-value row-value)))
(def (ui-sized-with size-value child-value)
  (ui-sized (vector-ref size-value 0) (vector-ref size-value 1) child-value))
(def (ui-input bordered-value background-value prefix-value title-value active-value (trailing #f))
  (vector 'input
          (vector bordered-value background-value prefix-value title-value active-value trailing)))
(def (ui-slot identity-value) (vector 'slot (vector identity-value)))
(def (ui-panel border-tone-value field-template-value title-value child-value)
  (vector 'panel (vector border-tone-value field-template-value title-value child-value)))
(def (ui-template background-value spans-value)
  (vector 'template (vector background-value (list->vector (map list->vector spans-value)))))
(def (ui-prompt field-value template-value)
  (vector 'prompt (vector field-value template-value)))
(def (ui-bar background-value divisor-value segments-value)
  (vector 'bar (vector background-value divisor-value (list->vector (map list->vector segments-value)))))
(def (ui-choice cases-value fallback-value)
  (vector 'choice (vector (list->vector (map list->vector cases-value)) fallback-value)))

(def (ui-binding-present identity)
  (string->symbol (string-append "binding-present/" (symbol->string identity))))
