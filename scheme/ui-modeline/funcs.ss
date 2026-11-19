;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list filter find)
        (only-in "../ui-elements/funcs.ss"
                 ui-bar ui-template ui-choice
                 ui-size-binding ui-size-cells ui-size-subtract)
        "./objects.ss")
(export marlin-ui-modeline-nodes marlin-ui-modeline-nodes-for marlin-ui-modeline-bar-for)

(def (modeline-node catalog entry)
  (def identity (car entry))
  (def caption (cadr entry))
  (def glyph (string (integer->char (cadr (assq identity (.get NerdModelineIcons. entries))))))
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

;; The text representation remains available to status consumers. In the bar,
;; split off the glyph so its color does not tint the separator or the label.
(def (modeline-label-spans entry label bold?)
  (def tone (.get entry tone))
  (def icon (assq (.get entry identity) (.get NerdModelineIcons. entries)))
  (def glyph (and icon (integer->char (cadr icon))))
  (def index (and glyph (string-index label glyph)))
  (filter (lambda (span) (not (string-empty? (cadddr span))))
    (if index
      (list (list 'literal tone bold? (substring label 0 index))
            (list 'literal (caddr icon) #f (string glyph))
            (list 'literal tone bold? (substring label (+ index 1) (string-length label))))
      (list (list 'literal tone bold? label)))))

;;; Turn the selected catalog into a generic flow. The host only supplies facts
;;; and values; ordering, colors, compact forms and retention stay here.
(def (marlin-ui-modeline-bar-for nodes)
  (def (mode-label wide compact)
    (ui-choice
      (list (list 'width-at-least-64 (ui-template 'none (list (list 'literal 'muted #f wide)))))
      (ui-template 'none (list (list 'literal 'muted #f compact)))))
  (def mode
    (ui-choice (list (list 'normal-mode (mode-label " NORMAL " " [N] ")))
               (mode-label " INSERT " " [I] ")))
  (let* ((slot (lambda (identity-value)
                 (or (find (lambda (entry) (eq? (.get entry identity) identity-value)) nodes)
                     (error "missing modeline entry" identity-value))))
         (width-fact (lambda (width-value)
                       (string->symbol (string-append "width-at-least-" (number->string width-value)))))
         (fixed (lambda (entry label bold?)
                  (ui-template 'none (modeline-label-spans entry label bold?))))
         (compact (lambda (entry full-value short-value)
                    (let (width (.get entry compact-below))
                      (if (zero? width) full-value
                        (ui-choice (list (list (width-fact width) full-value)) short-value)))))
         (dynamic (lambda (entry template-value value-name tail-value)
                    (let* ((tone-value (.get entry tone))
                           (parts (string-split template-value #\{))
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
                      (ui-template 'none
                        (append (modeline-label-spans entry before #f)
                                (list value-span)
                                (if (string-empty? after) '()
                                  (list (list 'literal tone-value #f after))))))))
         (activity (lambda (entry)
                     (compact entry
                       (fixed entry (string-append (.get entry label) " ") #f)
                       (fixed entry (string-append (.get entry compact) " ") #f))))
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
                    (let* ((id (.get entry identity))
                           (min-width (.get entry min-width)) (priority (.get entry keep-priority)))
                      (case id
                        ((modeline/left/brand)
                         (list (list 'left 0 0 #f
                                 (compact entry
                                   (fixed entry (string-append " " (.get entry label) " ") #t)
                                   (fixed entry (string-append " " (.get entry compact) " ") #t)))))
                        ((modeline/left/activity)
                         (list (list 'left 0 0 #f activity-view)))
                        ((modeline/left/model modeline/right/turns modeline/right/session)
                         (let* ((side (if (eq? id 'modeline/left/model) 'left 'right))
                                (value-name (case id ((modeline/left/model) 'model)
                                                   ((modeline/right/turns) 'turns) (else 'session)))
                                (tail-value (eq? id 'modeline/right/session))
                                (condition (and (eq? id 'modeline/right/turns) 'has-turns)))
                           (list (list side priority min-width condition
                                   (compact entry
                                     (dynamic entry (.get entry label) value-name tail-value)
                                     (dynamic entry (.get entry compact) value-name tail-value))))))
                        (else '())))) nodes))))
    (ui-bar 'bar 3 (append segments (list (list 'left 0 0 #f mode))))))
