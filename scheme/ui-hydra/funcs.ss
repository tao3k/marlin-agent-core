;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


;;; A body owns its heads, hint and foreign-key policy.
(import (only-in :clan/poo/object .get .o seal-object!)
        (only-in :std/list/list delete-duplicates/hash duplicates filter length<=n? take)
        (only-in "../ui-elements/funcs.ss" ui-text ui-key-hint-table)
        (only-in "./objects.ss" UiHydra.))
(export marlin-ui-hydra-entries marlin-ui-hydra-command-entries marlin-ui-hydra-key-bindings
        make-marlin-ui-hydra hydra-color-exit hydra-color-foreign hydra-head hydra-head-with-column)

(def (checked-heads heads)
  ;; Bound the traversal before checking proper list termination.
  (unless (and (pair? heads) (length<=n? heads 16) (list? heads))
    (error "invalid hydra heads"))
  (for-each
    (lambda (head)
      (unless (and (length<=n? head 5) (list? head) (= (length head) 5)
                   (symbol? (car head))
                   (or (and (not (cadr head)) (cadddr head))
                       (and (symbol? (cadr head)) (not (eq? (cadr head) 'exit)))
                       (let (command (cadr head))
                         (and (length<=n? command 5) (list? command)
                              (= (length command) 5)
                              (string? (car command)) (string? (cadr command))
                              (eq? (caddr command) 'none)
                              (symbol? (cadddr command)))))
                   (or (not (caddr head))
                       (and (string? (caddr head)) (<= (string-length (caddr head)) 64)))
                   (boolean? (cadddr head))
                   (let (column (list-ref head 4))
                     (or (not column) (and (string? column) (<= 1 (string-length column) 32)))))
        (error "invalid hydra head" head)))
    heads)
  (let (repeated (duplicates (map car heads) eq?))
    (unless (null? repeated) (error "duplicate hydra key" (caar repeated))))
  heads)

(def (checked-hydra hydra)
  (let ((identity-value (.get hydra identity)) (hint (.get hydra hint)))
    (unless (and (symbol? identity-value)
                 (let (name (symbol->string identity-value))
                   (and (<= 1 (string-length name) 20)
                        (andmap (lambda (c) (or (char<=? #\a c #\z)
                                                (char<=? #\0 c #\9) (char=? c #\-)))
                                (string->list name))))
                 (string? hint) (<= 1 (string-length hint) 2000)
                 (memq (.get hydra foreign) '(exit warn run)))
      (error "invalid hydra body"))
    (checked-heads (.get hydra heads))
    (when (and (memq (.get hydra foreign) '(warn run))
               (not (ormap cadddr (.get hydra heads))))
      (error "hydra must have an exit head"))
    hydra))

(def (hydra-format-doc title heads)
  (if (or (zero? (string-length title)) (not (char=? (string-ref title 0) #\newline)))
    title
    (let ((output (open-output-string)) (length-value (string-length title))
          (keys (map (lambda (head) (symbol->string (car head))) heads)))
      (let loop ((index 1))
        (when (< index length-value)
          (let (character (string-ref title index))
            (cond
              ((char=? character #\^) (loop (+ index 1)))
              ((and (char=? character #\\) (< (+ index 1) length-value)
                    (char=? (string-ref title (+ index 1)) #\^))
               (write-char #\^ output) (loop (+ index 2)))
              ((char=? character #\_)
               (let (end (string-index title #\_ (+ index 1)))
                 (cond ((and end (= end (+ index 1)))
                        (write-char #\_ output) (loop (+ end 1)))
                       ((and end (member (substring title (+ index 1) end) keys))
                        (display (substring title (+ index 1) end) output) (loop (+ end 1)))
                       (else (write-char character output) (loop (+ index 1))))))
              (else (write-char character output) (loop (+ index 1)))))))
      (get-output-string output))))

(def (make-marlin-ui-hydra identity-value title-value foreign-value heads-value
                          hint-mode-value key-value)
  (unless (and (string? title-value) (memq hint-mode-value '(auto none))
               (<= 1 (string-length title-value) 2000))
    (error "invalid hydra title"))
  (checked-heads heads-value)
  (checked-hydra
    (seal-object! (.o (:: @ UiHydra.) identity: identity-value
                      hint: (hydra-format-doc title-value heads-value)
                      hint-mode: hint-mode-value key: key-value
                      foreign: foreign-value heads: heads-value))))

;;; Colors follow Hydra's two independent decisions: exit after a head, and
;;; what an unrelated key does while the transient map is active.
(def (hydra-color-exit color)
  (case color
    ((red amaranth pink) #f)
    ((blue teal) #t)
    (else (error "invalid hydra color" color))))
(def (hydra-color-foreign color)
  (case color
    ((red blue) 'exit)
    ((amaranth teal) 'warn)
    ((pink) 'run)
    (else (error "invalid hydra color" color))))

;;; A nil command always exits, including in a continuing body.
(def (hydra-head key command label exit-value)
  (def caption
    (if (equal? label "")
      (cond ((symbol? command) (symbol->string command))
            ((pair? command) (cadr command))
            (else "quit"))
      label))
  (list key command caption (if command exit-value #t) #f))

(def (hydra-head-with-column head column)
  (append (take head 4) (list column)))

;;; The result is a typed native keymap entry, not serialized text.
(def (checked-catalog catalog)
  (let (entries (.get catalog entries))
    (unless (and (length<=n? entries 8) (list? entries))
      (error "too many hydras"))
    (for-each checked-hydra entries)
    (let (repeated (duplicates (map (lambda (body) (.get body identity)) entries) eq?))
      (unless (null? repeated)
        (error "duplicate hydra identity" (caar repeated))))
    entries))

(def (hydra-columns title heads)
  ;; Like Hydra's :column, a heading applies until another one is declared.
  (def entries
    (let loop ((pending heads) (column title))
      (if (null? pending) '()
        (let* ((head (car pending)) (next (or (list-ref head 4) column))
               (rest (loop (cdr pending) next)))
          (if (caddr head)
            (cons (list next (symbol->string (car head)) (caddr head) 'normal) rest)
            rest)))))
  (map (lambda (name)
         (list name (map cdr (filter (lambda (entry) (equal? name (car entry))) entries))))
       (delete-duplicates/hash (map car entries))))

(def (project-hydra hydra)
  (def title (.get hydra hint))
  (def automatic? (eq? (.get hydra hint-mode) 'auto))
  (def columns (and automatic? (hydra-columns title (.get hydra heads))))
  (def hint (if automatic? (ui-key-hint-table columns) (ui-text (list (cons 'normal title)))))
  (def warning
    (if (pair? columns)
      (ui-key-hint-table
        (cons (list (string-append "Unknown key · " (caar columns)) (cadar columns))
              (cdr columns)))
      (ui-text (list '(warning . "Unknown key · ") (cons 'normal title)))))
  (vector (.get hydra identity) hint (.get hydra foreign)
          (list->vector
            (map (lambda (head)
                   (vector (car head)
                           (cond ((not (cadr head)) 'exit)
                                 ((symbol? (cadr head)) (cadr head))
                                 (else (string->symbol (caadr head))))
                           (or (caddr head) "") (cadddr head)))
                 (.get hydra heads)))
          warning))

(def (marlin-ui-hydra-entries catalog)
  (list->vector (map project-hydra (checked-catalog catalog))))

(def (hydra-command-id identity-value)
  (string-append "hydra-" (symbol->string identity-value)))

(def (marlin-ui-hydra-key-bindings catalog)
  (filter-map
    (lambda (body)
      (let ((keys (.get body key)) (id (.get body identity)))
        (and keys
             (list 'normal keys (string->symbol (hydra-command-id id))
                   (string-append (symbol->string id) " group")))))
    (checked-catalog catalog)))

(def (marlin-ui-hydra-command-entries catalog)
  (foldr
    (lambda (body tail)
      (let (name (symbol->string (.get body identity)))
        (append (filter-map (lambda (head)
                              (let (command (cadr head))
                                (and (list? command) command)))
                            (.get body heads))
                (list (list (hydra-command-id (.get body identity))
                            (string-append "Open " name " command group")
                            'none 'enter-transient name))
                tail)))
    '()
    (checked-catalog catalog)))
