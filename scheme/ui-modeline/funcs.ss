;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list find)
        (only-in "../ui-elements/funcs.ss"
                 ui-bar ui-template ui-choice ui-size-binding ui-size-cells ui-size-subtract)
        "./objects.ss")
(export marlin-ui-modeline-nodes marlin-ui-modeline-nodes-for marlin-ui-modeline-bar-for)

(def (modeline-node catalog entry)
  (def identity (car entry))
  (def caption (cadr entry))
  (def glyph (string (integer->char (cdr (assq identity (.get NerdModelineIcons. entries))))))
  (def activity?
    (or (eq? identity 'modeline/left/activity)
        (string-prefix? "modeline/state/" (symbol->string identity))))
  (def dynamic? (memq identity '(modeline/left/model modeline/right/turns modeline/right/session)))
  (def prefix (if activity? (.get catalog activity-separator) ""))
  (.o identity
      label: (if dynamic? (string-append " " glyph " " caption " ")
               (string-append prefix glyph " " caption))
      compact: (if dynamic? (string-append glyph " " caption) (string-append prefix glyph))
      tone: (caddr entry) active-tone: (cadddr entry)
      min-width: (list-ref entry 4) compact-below: (list-ref entry 5)
      keep-priority: (list-ref entry 6)))

(def (marlin-ui-modeline-nodes-for profile)
  (def catalog
    (case profile
      ((doom-one) DoomModeline)
      ((pi) PiModeline.)
      (else (error "unknown UI modeline profile"))))
  (map (cut modeline-node catalog <>) (.get catalog slots)))

(def (marlin-ui-modeline-nodes)
  (marlin-ui-modeline-nodes-for 'doom-one))

;;; Turn the selected catalog into a generic flow. The host only supplies facts
;;; and values; ordering, colors, compact forms and retention stay here.
(def (marlin-ui-modeline-bar-for nodes)
  (let* ((slot (lambda (identity-value)
                 (or (find (lambda (entry) (eq? (.get entry identity) identity-value)) nodes)
                     (error "missing modeline entry" identity-value))))
         (width-fact (lambda (width-value)
                       (string->symbol (string-append "width-at-least-" (number->string width-value)))))
         (fixed (lambda (text-value tone-value background-value bold-value)
                  (ui-template background-value (list (list 'literal tone-value bold-value text-value)))))
         (compact (lambda (entry full-value short-value)
                    (ui-choice (list (list (width-fact (.get entry compact-below)) full-value)) short-value)))
         (dynamic (lambda (template-value value-name tone-value tail-value)
                    (let* ((parts (string-split template-value #\{))
                           (before (car parts))
                           (suffix (if (and (= (length parts) 2)
                                            (string-prefix? "value}" (cadr parts)))
                                     (cadr parts)
                                     (error "missing modeline value placeholder" template-value)))
                           (after (substring suffix 6 (string-length suffix)))
                           (value-span (if tail-value
                                         (list 'tail tone-value #f value-name
                                           (ui-size-subtract (ui-size-binding 'bar-right-budget)
                                             (ui-size-cells (+ (string-length before) (string-length after)))))
                                         (list 'binding tone-value #f value-name))))
                      (ui-template 'none (list (list 'literal tone-value #f before)
                                              value-span (list 'literal tone-value #f after))))))
         (activity (lambda (entry)
                     (let (tone-value (.get entry tone))
                       (compact entry
                         (fixed (string-append (.get entry label) " ") tone-value 'none #f)
                         (fixed (string-append (.get entry compact) " ") tone-value 'none #f)))))
         (activity-view
           (ui-choice
             (map (lambda (name-value)
                    (let* ((name (symbol->string name-value))
                           (entry (slot (string->symbol (string-append "modeline/state/" name)))))
                      (list (string->symbol (string-append "activity-" name)) (activity entry))))
                  '(online offline unknown runtime workspace recovery continuation history working reading))
             (activity (slot 'modeline/left/activity))))
         (segments
           (apply append
             (map (lambda (entry)
                    (let* ((id (.get entry identity)) (tone-value (.get entry tone))
                           (min-width (.get entry min-width)) (priority (.get entry keep-priority)))
                      (case id
                        ((modeline/left/brand)
                         (list (list 'left 0 0 #f
                                 (compact entry
                                   (fixed (string-append " " (.get entry label) " ") tone-value 'none #t)
                                   (fixed (string-append " " (.get entry compact) " ") tone-value 'none #t)))))
                        ((modeline/left/activity) (list (list 'left 0 0 #f activity-view)))
                        ((modeline/left/model modeline/right/turns modeline/right/session)
                         (let* ((side (if (eq? id 'modeline/left/model) 'left 'right))
                                (value-name (case id ((modeline/left/model) 'model)
                                                   ((modeline/right/turns) 'turns) (else 'session)))
                                (tail-value (eq? id 'modeline/right/session))
                                (condition (and (eq? id 'modeline/right/turns) 'has-turns)))
                           (list (list side priority min-width condition
                                   (compact entry
                                     (dynamic (.get entry label) value-name tone-value tail-value)
                                     (dynamic (.get entry compact) value-name tone-value tail-value))))))
                        (else '())))) nodes))))
    (ui-bar 'bar 3 segments)))
