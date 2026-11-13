;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; One body owns a trigger, heads, hint and foreign-key policy. A head's
;;; exit flag determines whether its command leaves the transient map.
(import (only-in :clan/poo/object .def .get .o seal-object!))
(export marlin-ui-hydra-keymap-entries
        marlin-ui-hydra-command-entries
        make-marlin-ui-hydra defui-hydra)

(.def UiHydra.)

(def (bounded-list? value limit)
  (let loop ((rest value) (remaining limit))
    (cond ((null? rest) #t)
          ((or (zero? remaining) (not (pair? rest))) #f)
          (else (loop (cdr rest) (- remaining 1))))))

(def (checked-heads heads)
  (unless (and (pair? heads) (bounded-list? heads 16))
    (error "invalid hydra heads"))
  (let loop ((rest heads) (seen '()))
    (unless (null? rest)
      (let (head (car rest))
        (unless (and (list? head) (= (length head) 4)
                     (symbol? (car head)) (not (memq (car head) seen))
                     (or (and (not (cadr head)) (cadddr head))
                         (let (command (cadr head))
                           (and (list? command) (= (length command) 5)
                                (string? (car command)) (string? (cadr command))
                                (eq? (caddr command) 'none)
                                (symbol? (cadddr command)))))
                     (string? (caddr head))
                     (<= 1 (string-length (caddr head)) 64)
                     (boolean? (cadddr head)))
          (error "invalid hydra head" head))
        (loop (cdr rest) (cons (car head) seen)))))
  heads)

(def (checked-hydra hydra)
  (let ((trigger (.get hydra trigger)) (hint (.get hydra hint)))
    (unless (and (symbol? (.get hydra identity))
                 (pair? trigger) (bounded-list? trigger 3) (andmap symbol? trigger)
                 (string? hint) (<= 1 (string-length hint) 2000)
                 (memq (.get hydra foreign) '(exit warn run)))
      (error "invalid hydra body"))
    (checked-heads (.get hydra heads))
    hydra))

(def (make-marlin-ui-hydra identity-value trigger-value title-value foreign-value heads-value auto-hint?)
  (unless (and (string? title-value) (boolean? auto-hint?)
               (<= 1 (string-length title-value) (if auto-hint? 64 2000)))
    (error "invalid hydra title"))
  (checked-heads heads-value)
  (def (hint-heads rest)
    (if (null? rest) ""
      (string-append (symbol->string (caar rest)) " " (caddar rest)
                     (if (null? (cdr rest)) "" "  ")
                     (hint-heads (cdr rest)))))
  (checked-hydra
    (seal-object! (.o (:: @ UiHydra.) identity: identity-value trigger: trigger-value
                      hint: (if auto-hint?
                              (string-append title-value ": " (hint-heads heads-value))
                              title-value)
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
   (list (symbol->string 'name) description 'none 'capability value)))

;;; Keyword datums match directly in patterns. The literals list is identifiers.
(defrules ui-hydra-head ()
  ((_ default-exit key command label)
   (list 'key (ui-hydra-command command) label default-exit))
  ((_ default-exit key command label color: color)
   (list 'key (ui-hydra-command command) label (hydra-color-exit 'color)))
  ((_ default-exit key command label exit: exit-value)
   (list 'key (ui-hydra-command command) label exit-value)))

;;; A declaration owns the trigger, command descriptors and per-head rules.
(defrules defui-hydra ()
  ((_ binding (identity-value (trigger-key ...) color: color hint: #f) title
      (head-key command-name label-value head-option ...) ...)
   (def binding
     (let (default-exit (hydra-color-exit 'color))
       (make-marlin-ui-hydra 'identity-value '(trigger-key ...) title
         (hydra-color-foreign 'color)
         (list (ui-hydra-head default-exit head-key command-name label-value head-option ...) ...)
         #f))))
  ((_ binding (identity-value (trigger-key ...) color: color) title
      (head-key command-name label-value head-option ...) ...)
   (def binding
     (let (default-exit (hydra-color-exit 'color))
       (make-marlin-ui-hydra 'identity-value '(trigger-key ...) title
         (hydra-color-foreign 'color)
         (list (ui-hydra-head default-exit head-key command-name label-value head-option ...) ...)
         #t)))))

;;; The result is a typed native keymap entry, not serialized text.
(def (checked-catalog catalog)
  (let (entries (.get catalog entries))
    (unless (bounded-list? entries 8) (error "too many hydras"))
    (let loop ((rest entries) (identities '()) (triggers '()))
      (unless (null? rest)
        (let* ((body (checked-hydra (car rest)))
               (identity-value (.get body identity))
               (trigger-value (.get body trigger)))
          (when (or (memq identity-value identities) (member trigger-value triggers))
            (error "duplicate hydra identity or trigger" identity-value))
          (loop (cdr rest) (cons identity-value identities) (cons trigger-value triggers)))))
    entries))

(def (project-hydra hydra)
  (checked-hydra hydra)
  (vector (list->vector (.get hydra trigger)) 'hydra
          (.get hydra hint) (.get hydra foreign)
          (list->vector
            (map (lambda (head)
                   (vector (car head)
                           (if (cadr head) (string->symbol (caadr head)) 'exit)
                           (caddr head) (cadddr head)))
                 (.get hydra heads)))
          (string-append "Unknown key · " (.get hydra hint))))

(def (marlin-ui-hydra-keymap-entries catalog)
  (list->vector (map project-hydra (checked-catalog catalog))))

(def (marlin-ui-hydra-command-entries catalog)
  (let loop ((groups (checked-catalog catalog)))
    (if (null? groups) '()
      (append (filter (lambda (entry) entry) (map cadr (.get (checked-hydra (car groups)) heads)))
              (loop (cdr groups))))))

