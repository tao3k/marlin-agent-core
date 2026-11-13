;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; One body owns heads, hint and foreign-key policy. A head's
;;; exit flag determines whether its command leaves the transient map.
(import (only-in :clan/poo/object .def .get .o seal-object!)
        (only-in :std/list/list duplicates filter length<=n?)
        (only-in "../ui-elements/objects.ss" ui-text))
(export marlin-ui-hydra-entries
        marlin-ui-hydra-command-entries
        make-marlin-ui-hydra defui-hydra)

(.def UiHydra.)

(def (checked-heads heads)
  ;; Bound the traversal before checking proper list termination.
  (unless (and (pair? heads) (length<=n? heads 16) (list? heads))
    (error "invalid hydra heads"))
  (for-each
    (lambda (head)
      (unless (and (length<=n? head 4) (list? head) (= (length head) 4)
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
                   (boolean? (cadddr head)))
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

(def (make-marlin-ui-hydra identity-value title-value foreign-value heads-value hint-mode)
  (unless (and (string? title-value) (memq hint-mode '(auto none))
               (<= 1 (string-length title-value) 2000))
    (error "invalid hydra title"))
  (checked-heads heads-value)
  (def visible-hint
    (if (eq? hint-mode 'none) ""
      (string-join
        (map (lambda (head)
               (let (label (caddr head))
                 (string-append (symbol->string (car head))
                                (if (string=? label "") "" (string-append " " label)))))
             (filter caddr heads-value))
        ", ")))
  (def formatted (hydra-format-doc title-value heads-value))
  (def hint-value
    (if (string=? visible-hint "") formatted
      (string-append formatted
                     (if (char=? (string-ref title-value 0) #\newline) "" ": ")
                     visible-hint ".")))
  (checked-hydra
    (seal-object! (.o (:: @ UiHydra.) identity: identity-value
                      hint: hint-value
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

(defrules ui-hydra-command ()
  ((_ #f) #f)
  ((_ (name description capability value))
   (list (symbol->string 'name) description 'none 'capability value))
  ((_ name) 'name))

;;; A nil command always exits, including in a continuing body.
(def (hydra-head key command label exit-value)
  (list key command label (if command exit-value #t)))

;;; Keyword datums match directly in patterns. The literals list is identifiers.
(defrules ui-hydra-head ()
  ((_ default-exit default-hint key command color: color)
   (hydra-head 'key (ui-hydra-command command) default-hint (hydra-color-exit 'color)))
  ((_ default-exit default-hint key command exit: exit-value)
   (hydra-head 'key (ui-hydra-command command) default-hint exit-value))
  ((_ default-exit default-hint key command label color: color)
   (hydra-head 'key (ui-hydra-command command) label (hydra-color-exit 'color)))
  ((_ default-exit default-hint key command label exit: exit-value)
   (hydra-head 'key (ui-hydra-command command) label exit-value))
  ((_ default-exit default-hint key command label)
   (hydra-head 'key (ui-hydra-command command) label default-exit))
  ((_ default-exit default-hint key command)
   (hydra-head 'key (ui-hydra-command command) default-hint default-exit)))

;;; A declaration owns the command descriptors and per-head rules.
(defrules defui-hydra ()
  ((_ binding (identity-value color: color hint: #f) title
      (head-key command-name head-option ...) ...)
   (def binding
     (let (default-exit (hydra-color-exit 'color))
       (make-marlin-ui-hydra 'identity-value title
         (hydra-color-foreign 'color)
         (list (ui-hydra-head default-exit #f head-key command-name head-option ...) ...)
         'auto))))
  ((_ binding (identity-value color: color hint: none) title
      (head-key command-name head-option ...) ...)
   (def binding
     (let (default-exit (hydra-color-exit 'color))
       (make-marlin-ui-hydra 'identity-value title
         (hydra-color-foreign 'color)
         (list (ui-hydra-head default-exit #f head-key command-name head-option ...) ...)
         'none))))
  ((_ binding (identity-value color: color) title
      (head-key command-name head-option ...) ...)
   (def binding
     (let (default-exit (hydra-color-exit 'color))
       (make-marlin-ui-hydra 'identity-value title
         (hydra-color-foreign 'color)
         (list (ui-hydra-head default-exit "" head-key command-name head-option ...) ...)
         'auto)))))

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

(def (project-hydra hydra)
  (checked-hydra hydra)
  (vector (.get hydra identity) (ui-text (list (cons 'normal (.get hydra hint)))) (.get hydra foreign)
          (list->vector
            (map (lambda (head)
                   (vector (car head)
                           (cond ((not (cadr head)) 'exit)
                                 ((symbol? (cadr head)) (cadr head))
                                 (else (string->symbol (caadr head))))
                           (or (caddr head) "") (cadddr head)))
                 (.get hydra heads)))
          (ui-text (list (cons 'warning "Unknown key · ") (cons 'normal (.get hydra hint))))))

(def (marlin-ui-hydra-entries catalog)
  (list->vector (map project-hydra (checked-catalog catalog))))

(def (marlin-ui-hydra-command-entries catalog)
  (foldr
    (lambda (body tail)
      (let (name (symbol->string (.get body identity)))
        (append (filter-map (lambda (head)
                              (let (command (cadr head))
                                (and (list? command) command)))
                            (.get body heads))
                (list (list (string-append "hydra-" name)
                            (string-append "Open " name " command group")
                            'none 'enter-transient name))
                tail)))
    '()
    (checked-catalog catalog)))

