;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list filter find)
        (only-in :std/string/misc string-trim)
        (only-in "../ui-elements/funcs.ss"
                 ui-bar ui-template ui-choice
                 ui-size-binding ui-size-cells ui-size-subtract)
        "./objects.ss")
(export marlin-ui-modeline-nodes marlin-ui-modeline-nodes-for marlin-ui-modeline-bar-for
        marlin-ui-modeline-preset-spec marlin-ui-modeline-visible-segments
        marlin-ui-modeline-shrink-path)

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
  ;; Doom Emacs window-number segment: rendered directly alongside the modal
  ;; state indicator ONLY when multiple windows are open (never "W1" on single).
  (def window-number
    (ui-template 'none
      (list (list 'literal 'accent #t " ")
            (list 'binding 'accent #t 'window-number)
            (list 'literal 'accent #t " "))))
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
    (ui-bar 'bar 3 (append segments
                           (list (list 'left 0 0 #f mode)
                                 (list 'left 0 0 'multi-window window-number))))))

;;; Return the declarative LHS/RHS segment layout for a Modern Doom Modeline
;;; preset ('main, 'dashboard, 'helm, or 'minimal).
(def (marlin-ui-modeline-preset-spec preset)
  (def entry
    (or (assq preset (.get ModernDoomModelinePresets. presets))
        (assq 'main (.get ModernDoomModelinePresets. presets))))
  (.o preset: (car entry)
      lhs: (cadr entry)
      rhs: (caddr entry)))

;;; Filter a preset's LHS/RHS segments by pane column budget, multi-window
;;; presence, and window focus (matching Doom Emacs doom-modeline-window-width-limit).
(def (marlin-ui-modeline-visible-segments preset width-cols multi-window? focused?)
  (def spec (marlin-ui-modeline-preset-spec preset))
  (def rules (.get ModernDoomSegmentRules. rules))
  (def (admit? seg)
    (def rule (assq seg rules))
    (def min-w (if rule (caddr rule) 0))
    (cond
      ((eq? seg 'window-number) multi-window?)
      ((not focused?)
       (memq seg '(bar window-state window-number buffer-info major-mode)))
      (else (or (<= width-cols 0) (>= width-cols min-w)))))
  (.o preset: (.get spec preset)
      lhs: (filter admit? (.get spec lhs))
      rhs: (filter admit? (.get spec rhs))))

;;; Doom Emacs `shrink-path` projection: collapses intermediate directory
;;; segments to their leading character while preserving root (~ or /) and
;;; the final leaf segment (e.g. "~/ghq/github.com/tao3k/marlin-agent-core/"
;;; => "~/g/g/t/marlin-agent-core/").
(def (marlin-ui-modeline-shrink-path raw-path)
  (def trimmed (string-trim raw-path))
  (if (or (string-empty? trimmed) (not (string-index trimmed #\/)))
    trimmed
    (let* ((trailing-slash? (and (> (string-length trimmed) 1)
                                 (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\/)))
           (body (if trailing-slash?
                   (substring trimmed 0 (- (string-length trimmed) 1))
                   trimmed))
           (parts (string-split body #\/))
           (total (length parts))
           (shrunk
             (let loop ((rest parts) (idx 0) (acc '()))
               (if (null? rest)
                 (reverse acc)
                 (let* ((seg (car rest))
                        (last? (= idx (- total 1)))
                        (first? (= idx 0))
                        (kept (cond
                                ((or last? (string-empty? seg)) seg)
                                ((and first? (or (string=? seg "~") (string=? seg ".") (string=? seg ".."))) seg)
                                ((<= (string-length seg) 1) seg)
                                ((and (char=? (string-ref seg 0) #\.) (> (string-length seg) 1))
                                 (substring seg 0 2))
                                (else (substring seg 0 1)))))
                   (loop (cdr rest) (+ idx 1) (cons kept acc))))))
           (joined (string-join shrunk "/")))
      (if trailing-slash? (string-append joined "/") joined))))
