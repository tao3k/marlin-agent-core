;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Compose a bounded bar from the admitted buffer catalog.
(import (only-in :clan/poo/object .get .o .ref .slot? object?)
        (only-in :std/list/list delete-duplicates/hash filter filter-map)
        (only-in "../ui-components/buffers.ss" marlin-ui-buffer-command-name)
        (only-in "../ui-elements/funcs.ss" ui-bar ui-choice ui-template)
        "./objects.ss")
(export marlin-ui-buffer-tabs marlin-ui-tabs-for-style
        marlin-ui-default-tab-style marlin-ui-tab-style? marlin-ui-tab-style-commands
        marlin-ui-task-tab-identity marlin-ui-task-tab-declarations
        marlin-ui-task-tabs marlin-ui-task-tab-bar-for)

(def tab-styles (.get TabStyles. entries))
(def marlin-ui-default-tab-style (caar tab-styles))

(def (marlin-ui-tab-style? identity)
  (and (assq identity tab-styles) #t))

(def (marlin-ui-tab-style-commands)
  (map (lambda (entry)
         (let (name (symbol->string (car entry)))
           (list name (cadr entry) 'none 'select-profile name)))
       tab-styles))

(def (short-label entry)
  (if (null? (cdddr entry)) (cadr entry) (cadddr entry)))

(def (width-fact cells)
  (string->symbol (string-append "width-at-least-" (number->string cells))))

(def (tab-width label icon?)
  (+ 3 (string-length label) (if icon? 2 0)))

(def (tab-choice cases fallback)
  ;; A later width must be smaller to be reachable. This also removes
  ;; duplicate facts when the short and full strips have the same width.
  (def reachable
    (foldl (lambda (entry accepted)
             (if (or (null? accepted) (< (car entry) (caar accepted)))
               (cons entry accepted)
               accepted))
           '() cases))
  ;; Adjacent widths with the same presentation only need the smaller bound.
  (def distinct
    (foldl (lambda (entry retained)
             (let (next (if (null? retained) fallback (cadar retained)))
               (if (equal? (cadr entry) next) retained (cons entry retained))))
           '() reachable))
  (if (null? distinct) fallback
    (ui-choice (map (lambda (entry) (list (width-fact (car entry)) (cadr entry))) distinct)
               fallback)))

(def (tab-presentation layout entry active? starts-group? icon)
  (def style (.get layout style))
  (def ink (if active? (.get style selected-ink) (.get style unselected-ink)))
  (def background (if active? (.get style selected-background) 'bar))
  (def mark (if active? (.get style selected-mark) " "))
  (def label (cadr entry))
  (def short (short-label entry))
  (def (template label-value divider glyph)
    (def prefix (if label-value (string-append mark " ") " "))
    (def suffix
      (if label-value
        (string-append label-value (if active? (.get style selected-end) " "))
        ""))
    (ui-template background
      (append
        (if divider (list (list 'literal 'muted #f divider)) '())
        (if glyph
          (list (list 'literal ink active? prefix)
                (list 'literal (or (and active? (.get style selected-icon-ink)) (caddr glyph))
                      #f (string (integer->char (cadr glyph))))
                (list 'literal ink active? (string-append " " suffix)))
          (list (list 'literal ink active? (string-append prefix suffix)))))))
  (def plain (template short #f #f))
  (def strip-cases
    (list (list (.get layout full-width)
                (template label (and starts-group? (.get style group-mark)) icon))
          (list (.get layout short-width) (template short #f icon))))
  (if active?
    ;; Reserve roughly half the strip for its neighbours before showing the
    ;; full selected label. The short form remains available at any width.
    (tab-choice
      (append strip-cases
              (list (list (* 2 (tab-width label icon)) (template label #f icon))
                    (list (tab-width short icon) (template short #f icon))))
      plain)
    (tab-choice
      (append strip-cases (list (list (.get layout labels-width) plain)))
      (if icon (template #f #f icon) plain))))

(def (tab-segments layout entry starts-group?)
  (def identity (car entry))
  (def name (symbol->string identity))
  (def command (string->symbol (marlin-ui-buffer-command-name identity)))
  (def icon (assq identity (.get layout icons)))
  (map (lambda (active?)
         (list 'left (if active? 0 3) 0
               (string->symbol
                 (string-append (if active? "binding-is/" "binding-is-not/")
                                "active-buffer/" name))
               (tab-presentation layout entry active? starts-group? icon) command))
       '(#t #f)))

(def (marlin-ui-buffer-tabs identities (declarations '()) (style-id marlin-ui-default-tab-style)
                            nerd-icons?: (nerd-icons? #t))
  (def entry (assq style-id tab-styles))
  (unless entry (error "unknown Centaur Tabs style" style-id))
  (def style (caddr entry))
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
      (delete-duplicates/hash (append '(session workspace packages) (map caddr entries)))))
  ;; Built-in labels and marks are single-cell text. Ratatui still measures
  ;; the painted spans and clips custom labels to the available terminal cells.
  (def (bar-width label-of icons? dividers)
    (foldl (lambda (group total)
             (foldl (lambda (entry width)
                      (+ width (tab-width (label-of entry)
                                          (and icons? (assq (car entry) icons)))))
                    total group))
           dividers groups))
  (def layout
    (.o style icons
        full-width: (bar-width cadr #t (max 0 (- (length groups) 1)))
        short-width: (bar-width short-label #t 0)
        labels-width: (quotient (+ 1 (bar-width short-label #f 0)) 2)))
  (def (group-segments group starts-group?)
    (append (tab-segments layout (car group) starts-group?)
            (apply append (map (lambda (entry) (tab-segments layout entry #f)) (cdr group)))))
  (ui-bar 'bar 1
    (if (null? groups) '()
      (append (group-segments (car groups) #f)
              (apply append (map (lambda (group) (group-segments group #t)) (cdr groups)))))))

(def (marlin-ui-tabs-for-style buffers style)
  (marlin-ui-buffer-tabs buffers '() style))

;;; ---------------------------------------------------------------------------
;;; Task-as-Tab Workbench Projection (RFC 40.183 §7.6)
;;; Pure functional projection of Durable `WorkbenchTabProjection`:
;;; Left Pinned Group (`Index`, non-closable) + Right Dynamic Group (`Active Task Tabs`).
;;; ---------------------------------------------------------------------------

(def (task-opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

(def (marlin-ui-task-tab-identity raw-id)
  (cond
   ((symbol? raw-id) raw-id)
   ((string? raw-id) (string->symbol raw-id))
   (else (error "invalid task tab identity" raw-id))))

(def (truncate-tab-label text limit)
  (if (<= (string-length text) limit)
    text
    (string-append (substring text 0 (max 1 (- limit 1))) "…")))

(def (task-tab-entry-declaration tab-obj group-id)
  (let* ((id (marlin-ui-task-tab-identity (.ref tab-obj 'id)))
         (raw-label (task-opt-ref tab-obj 'label (symbol->string id)))
         (kind (task-opt-ref tab-obj 'kind 'task))
         (mode (task-opt-ref tab-obj 'mode 'active))
         (status (task-opt-ref tab-obj 'status 'todo))
         (done-c (task-opt-ref tab-obj 'checklist-done 0))
         (total-c (task-opt-ref tab-obj 'checklist-total 0))
         (mode-suffix (or (let ((p (assq mode (.get TaskTabBadges. mode-suffix))))
                            (and p (cdr p)))
                          ""))
         (status-prefix (if (eq? kind 'index)
                          ""
                          (or (let ((p (assq status (.get TaskTabBadges. status-mark))))
                                (and p (cdr p)))
                              "")))
         (cookie (if (and (not (eq? kind 'index)) (integer? total-c) (> total-c 0))
                   (string-append " [" (number->string done-c) "/" (number->string total-c) "]")
                   ""))
         (full-label (if (eq? kind 'index)
                       raw-label
                       (string-append status-prefix
                                      (truncate-tab-label raw-label 22)
                                      cookie
                                      mode-suffix)))
         (short-text (if (eq? kind 'index)
                       "Idx"
                       (string-append (truncate-tab-label raw-label 10) mode-suffix))))
    (list id full-label group-id short-text)))

(def (marlin-ui-task-tab-declarations projection)
  (let* ((pinned (task-opt-ref projection 'pinned-tabs '()))
         (dynamic (task-opt-ref projection 'dynamic-tabs '()))
         (pinned-decls (map (lambda (t) (task-tab-entry-declaration t 'task-index)) pinned))
         (dynamic-decls
          (map (lambda (t)
                 (let ((mode (task-opt-ref t 'mode 'active)))
                   (task-tab-entry-declaration
                    t (if (eq? mode 'peek-readonly) 'peek-tasks 'active-tasks))))
               dynamic)))
    (append pinned-decls dynamic-decls)))

(def (marlin-ui-task-tabs projection
                          (style-id marlin-ui-default-tab-style)
                          nerd-icons?: (nerd-icons? #t))
  (let* ((declarations (marlin-ui-task-tab-declarations projection))
         (identities (map car declarations)))
    (marlin-ui-buffer-tabs identities declarations style-id nerd-icons?: nerd-icons?)))

(def (marlin-ui-task-tab-bar-for projection nerd-icons?: (nerd-icons? #t))
  (lambda (_buffer-identities style-id)
    (marlin-ui-task-tabs projection style-id nerd-icons?: nerd-icons?)))
