;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Compose a bounded bar from the admitted buffer catalog.
(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list filter filter-map)
        (only-in "../ui-components/buffers.ss" marlin-ui-buffer-command-name)
        (only-in "../ui-elements/funcs.ss" ui-bar ui-choice ui-template)
        "./objects.ss")
(export marlin-ui-buffer-tabs marlin-ui-tabs-for-profile)

(def (short-label entry)
  (if (null? (cdddr entry)) (cadr entry) (cadddr entry)))

(def (width-fact cells)
  (string->symbol (string-append "width-at-least-" (number->string cells))))

(def (tab-presentation layout entry active? starts-group? icon)
  (def style (.get layout style))
  (def ink (if active? (.get style selected-ink) (.get style unselected-ink)))
  (def background (if active? (.get style selected-background) 'bar))
  (def mark (if active? (.get style selected-mark) " "))
  (def label (cadr entry))
  (def (template label-value divider icon-value)
    (ui-template background
      (list (list 'literal ink active?
                  (string-append divider mark " " icon-value label-value " ")))))
  ;; Keep group marks inside a tab so an omitted group cannot leave a divider.
  (ui-choice
    (list (list (.get layout full-width)
                (template label (if starts-group? (.get style group-mark) "") icon))
          (list (.get layout short-width) (template (short-label entry) "" icon)))
    (template (short-label entry) "" "")))

(def (tab-segments layout entry starts-group?)
  (def identity (car entry))
  (def name (symbol->string identity))
  (def command (string->symbol (marlin-ui-buffer-command-name identity)))
  (def icon-value (assq identity (.get layout icons)))
  (def icon (if icon-value (string (integer->char (cdr icon-value)) #\space) ""))
  (map (lambda (active?)
         (list 'left (if active? 0 3) 0
               (string->symbol
                 (string-append (if active? "binding-is/" "binding-is-not/")
                                "active-buffer/" name))
               (tab-presentation layout entry active? starts-group? icon) command))
       '(#t #f)))

(def (marlin-ui-buffer-tabs identities (declarations '()) (profile 'doom-one)
                            nerd-icons?: (nerd-icons? #t))
  (def style (if (eq? profile 'pi) PiTabs. DoomTabs.))
  (def icons (if nerd-icons? (.get NerdTabIcons. entries) '()))
  (def builtin-tabs (.get BuiltinTabs. entries))
  (def builtin-ids (filter (cut memq <> identities) (map car builtin-tabs)))
  (def added-ids (filter (lambda (id) (not (assq id builtin-tabs))) identities))
  (def entries
    (map (lambda (id) (or (assq id declarations) (assq id builtin-tabs)
                         (list id (symbol->string id) 'packages)))
         (append builtin-ids added-ids)))
  (def groups
    (filter-map
      (lambda (group)
        (let (members (filter (lambda (entry) (eq? (caddr entry) group)) entries))
          (and (pair? members) members)))
      '(session workspace packages)))
  ;; Built-in labels and marks are single-cell text. Ratatui still measures
  ;; the painted spans and clips custom labels to the available terminal cells.
  (def (bar-width label-of dividers)
    (foldl (lambda (group total)
             (foldl (lambda (entry width)
                      (+ width 3 (string-length (label-of entry))
                         (if (assq (car entry) icons) 2 0)))
                    total group))
           dividers groups))
  (def layout
    (.o style icons
        full-width: (width-fact (bar-width cadr (max 0 (- (length groups) 1))))
        short-width: (width-fact (bar-width short-label 0))))
  (def (group-segments group starts-group?)
    (append (tab-segments layout (car group) starts-group?)
            (apply append (map (lambda (entry) (tab-segments layout entry #f)) (cdr group)))))
  (ui-bar 'bar 1
    (if (null? groups) '()
      (append (group-segments (car groups) #f)
              (apply append (map (lambda (group) (group-segments group #t)) (cdr groups)))))))

(def (marlin-ui-tabs-for-profile buffers profile)
  (marlin-ui-buffer-tabs buffers '() profile))
