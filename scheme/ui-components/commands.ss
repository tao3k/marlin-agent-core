;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Names, descriptions, argument grammar and capability bindings have one
;;; selected Scheme owner. The native host admits the capabilities and effects.
(import (only-in :clan/poo/object .def .o .get .ref .mix seal-object!)
        (only-in :std/list/list duplicates)
        (only-in "../ui-elements/objects.ss" ui-template ui-choice)
        (only-in "./bounds.ss" marlin-ui-bounded-list?)
        (only-in "./buffers.ss" marlin-ui-buffer-command-entries
                 marlin-ui-buffer-alias-command-entries))
(export marlin-ui-command-catalog marlin-ui-command-entries
        marlin-ui-candidate-templates
        marlin-ui-command-history? marlin-ui-record-command-history
        marlin-ui-command-entries? marlin-ui-command-candidates
        marlin-ui-browse-command marlin-ui-profile-command-entries
        marlin-ui-command-catalog-with-profiles marlin-ui-command-catalog-with-extensions)

;;; A native template contains typed literal and binding spans, not encoded text.
(def marlin-ui-candidate-templates
  (ui-choice
    (list (list 'selected
                (ui-template 'bar '((literal accent #f " › ") (binding info #t name)
                                    (literal normal #f "  ") (binding muted #f description)))))
    (ui-template 'none '((literal accent #f "   ") (binding normal #f name)
                         (literal normal #f "  ") (binding muted #f description)))))

(.def UiCommandCatalog.
  entries: (append (marlin-ui-buffer-alias-command-entries)
                   '(("models" "List configured models" none models #f)
                     ("model" "Select a model alias" required model #f))))
;;; Journal recovery remains a storage operation, not a Context source.
(.def JournalCommands.
  (entries => append '(("sessions" "Browse journal IDs; next for another page" optional sessions #f)
                       ("session" "Recover a quoted journal ID" required session #f)
                       ("inspect" "Read a quoted journal ID" required inspect #f))))
(.def WorkspaceCommands.
  (entries => append '(("status" "Show workspace status" none workspace "status")
                       ("diff" "Show unstaged or staged changes" optional workspace "diff")
                       ("context-budget" "Show request budget" none context-budget #f)
                       ("runtime" "Show runtime policy" none runtime #f)
                       ("list-processes" "List managed Runtime processes"
                        none select-buffer "processes")
                       ("kill-process" "Stop a managed process by generation"
                        required kill-process #f)
                       ("switch-themes" "Theme: " none open-menu "theme-")
                       ("switch-buffer" "Switch to: " none open-menu "buffer-"))
                     (marlin-ui-buffer-command-entries)
                     '(("toggle-thinking" "Expand or fold thinking (N from newest)"
                        optional toggle-thinking #f)
                       ("clear" "Clear displayed transcript" none clear #f))))
;;; The inherited slot computes one ordered catalog when the selected module
;;; reads it. No additional loader or imperative append registry is needed.
(def marlin-ui-command-catalog
  (seal-object! (.mix WorkspaceCommands. JournalCommands. UiCommandCatalog.)))

;;; Preserve the anchor and its tail while copying the prefix only once.
(def (insert-before-command entries anchor additions)
  (let insert ((rest entries))
    (cond ((null? rest) (error "missing command catalog anchor" anchor))
          ((string=? (caar rest) anchor) (append additions rest))
          (else (cons (car rest) (insert (cdr rest)))))))

;;; The selected theme scope owns profiles. Components only turn its bounded
;;; entries into native-admissible command descriptors.
(def (marlin-ui-profile-command-entries profiles)
  (unless (and (marlin-ui-bounded-list? profiles 8)
               (andmap (lambda (entry)
                         (and (marlin-ui-bounded-list? entry 3)
                              (= (length entry) 3)
                              (symbol? (car entry))
                              (string? (cadr entry))
                              (<= (string-length (cadr entry)) 256)
                              (let (name (symbol->string (car entry)))
                                (and (<= 1 (string-length name) 26)
                                     (string=? name (string-downcase name))))))
                       profiles))
    (error "invalid UI profile catalog"))
  (unless (null? (duplicates (map car profiles) eq?))
    (error "duplicate UI profile identity"))
  (map (lambda (entry)
         (let (name (symbol->string (car entry)))
           (list (string-append "theme-" name) (cadr entry)
                 'none 'select-profile name)))
       profiles))

(def (marlin-ui-command-catalog-with-profiles catalog profiles)
  ;; Validate before constructing the lazy POO slot; malformed profiles must
  ;; fail at this boundary even if the caller never reads entries.
  (let (generated (marlin-ui-profile-command-entries profiles))
    (.o entries: (insert-before-command (.get catalog entries) "switch-buffer" generated))))

(def (command-entry? entry)
  (and (marlin-ui-bounded-list? entry 5)
       (= (length entry) 5)
       (string? (car entry)) (<= 1 (string-length (car entry)) 32)
       (string? (cadr entry)) (<= (string-length (cadr entry)) 256)
       (memq (caddr entry) '(none required optional))
       (memq (cadddr entry)
             '(models model sessions session inspect history workspace context-budget
               runtime open-menu select-profile select-buffer enter-transient
               kill-process toggle-thinking clear))
       (let (value (car (cddddr entry)))
         (or (not value) (and (string? value) (<= 1 (string-length value) 32))))))

;;; Admission is bounded before map/filter can traverse a supplied catalog.
(def (marlin-ui-command-entries? entries)
  (and (marlin-ui-bounded-list? entries 256) (andmap command-entry? entries)))

(def (marlin-ui-command-catalog-with-extensions catalog extensions)
  (unless (marlin-ui-command-entries? extensions) (error "invalid extension commands"))
  (let* ((base (.get catalog entries))
         (names (map car (append base extensions))))
    (let (repeated (duplicates names))
      (unless (null? repeated) (error "duplicate UI command" (caar repeated))))
    (.o entries: (insert-before-command base "status" extensions))))

(def (marlin-ui-command-entries catalog)
  (let (entries (.get catalog entries))
    (unless (marlin-ui-command-entries? entries) (error "invalid UI command catalog"))
    (map (lambda (entry)
           (list (string-copy (car entry)) (string-copy (cadr entry))
                 (caddr entry) (cadddr entry)
                 (let (value (car (cddddr entry)))
                   (if value (string-copy value) #f))))
         entries)))

;;; A browse result is a presentation proposal, checked again by the host.
(def command-history-limit 256)
(def command-history-duplicate-boost 10)
(def command-history-decay-span 100.0)
(def (marlin-ui-command-history? value)
  (and (marlin-ui-bounded-list? value command-history-limit)
       (andmap (lambda (name)
                 (and (string? name) (<= (string-length name) 32)))
               value)))
(def (marlin-ui-record-command-history history-value name-value)
  (unless (and (marlin-ui-command-history? history-value)
               (string? name-value) (<= 1 (string-length name-value) 32))
    (error "invalid UI command history"))
  (cons name-value
        (if (>= (length history-value) command-history-limit)
          (take history-value (- command-history-limit 1))
          history-value)))

(def (command-subsequence? query name)
  ;; The installed string-index wrapper allocates on each character lookup;
  ;; completion scans every candidate, so retain the allocation-free scan.
  (let ((needle-length (string-length query))
        (name-length (string-length name)))
    (let loop ((needle 0) (position 0))
      (cond ((= needle needle-length) #t)
            ((= position name-length) #f)
            ((char=? (string-ref query needle) (string-ref name position))
             (loop (+ needle 1) (+ position 1)))
            (else (loop needle (+ position 1)))))))

;;; The bounded history uses the same decay positions on every query.
(def command-history-decay
  (list->vector
    (map (lambda (index)
           (inexact->exact (round (* command-history-duplicate-boost
                                     (exp (/ (- index) command-history-decay-span))))))
         (iota command-history-limit))))

(def (marlin-ui-command-candidates entries query history)
  (unless (and (marlin-ui-command-entries? entries)
               (string? query) (<= (string-length query) 32)
               (marlin-ui-command-history? history))
    (error "invalid UI command candidate request"))
  ;; Vertico's history-length-alpha ordering: recent history first, repeated
  ;; uses receive an exponentially decaying boost, then length and alphabetic
  ;; order. The first history item is never displaced by duplicates.
  (let (ranks (make-hash-table))
    (let loop ((rest history) (index 0))
      (unless (null? rest)
        (let* ((name (car rest))
               (prior (hash-get ranks name))
               (rank (if prior
                       (- prior (vector-ref command-history-decay index))
                       (if (zero? index)
                         (- (+ 1 (* command-history-limit command-history-duplicate-boost)))
                         index))))
          (hash-put! ranks name rank)
          (loop (cdr rest) (+ index 1)))))
    (let* ((names (filter-map (lambda (entry)
                                (let (name (car entry))
                                  (and (command-subsequence? query name) name)))
                              entries))
           (ordered
             (list-sort
               (lambda (left right)
                 (let ((lrank (hash-get ranks left)) (rrank (hash-get ranks right)))
                   (cond ((and lrank rrank)
                          (if (= lrank rrank) (string<? left right) (< lrank rrank)))
                         (lrank #t)
                         (rrank #f)
                         ((= (string-length left) (string-length right))
                          (string<? left right))
                         (else (< (string-length left) (string-length right))))))
               names)))
      ;; Vertico gives an exact field match priority over the history sort.
      (if (member query ordered)
        (cons query (remove (lambda (name) (string=? name query)) ordered))
        ordered))))

(def (marlin-ui-browse-command entries event)
  (let* ((query (.ref event 'query))
         (previous (.ref event 'previous))
         (direction (.ref event 'direction))
         (steps (.ref event 'steps))
         (history (.ref event 'history))
         (allowed (.ref event 'allowed)))
    (unless (and (marlin-ui-command-entries? entries)
                 (string? query) (<= 0 (string-length query) 32)
                 (or (not previous)
                     (and (string? previous) (<= (string-length previous) 32)))
                 (memq direction '(next previous))
                 (integer? steps) (<= 1 steps 16)
                 (or (not allowed)
                     (and (marlin-ui-bounded-list? allowed 256)
                          (andmap (lambda (name)
                                    (and (string? name) (<= (string-length name) 32)))
                                  allowed))))
      (error "invalid command browse event or catalog"))
    (let* ((all (marlin-ui-command-candidates entries query history))
           (matches (if allowed
                      (filter (lambda (name) (member name allowed)) all)
                      all))
           (count (length matches))
           (tail (and previous (member previous matches)))
           (found (and tail (- count (length tail))))
           (choice (if (zero? count) #f
                     (list-ref matches
                       (modulo (if found
                                 (+ found (if (eq? direction 'next) steps (- steps)))
                                 (if (eq? direction 'next) (- steps 1) (- count steps)))
                               count)))))
      (.o kind: 'command-selection name: choice))))
