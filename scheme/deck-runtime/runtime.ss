;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Context identities route messages to resident Gerbil mailbox Actors.

(import (only-in :clan/poo/object .get .ref .o)
        :marlin-agent-core/deck-runtime/mailbox
        :marlin-agent-core/deck-runtime/timers
        :marlin-agent-core/deck-runtime/ingress)

(export open-deck-runtime check-deck-kernel!)

(def (context-actor-key row)
  (map (lambda (name)
         (let (value (.ref row name)) (if (string? value) (string-copy value) value)))
       '(bundle organization project worktree actor session)))

(def (open-deck-runtime admit-context encode-scopes)
  (def (admit-message request)
    (def admitted (admit-context request))
    ;; Force the lazy identity before publishing this message's completion.
    (unless (string? (.get admitted semantic-digest))
      (error "Context admission did not produce a Scope identity"))
    admitted)
  (def mailboxes (open-deck-mailboxes car (lambda (message) (context-actor-key (cdr message)))
                                             (lambda (message) (admit-message (cdr message)))))
  (def modules (make-hash-table-eqv))
  (def ingress #f)
  (def (activate-module! id)
    (when (hash-key? modules id) (error "Deck Module is already active"))
    ((.get mailboxes activate) id)
    (hash-put! modules id (open-deck-timers id (.get mailboxes notify)))
    #t)
  (activate-module! 1)
  (def (retire-module! id)
    (def timers (or (hash-get modules id) (error "Deck Module is not active")))
    (def cancelled ((.get timers close)))
    (def counts ((.get mailboxes retire) id))
    (hash-remove! modules id)
    (vector counts (u64vector cancelled)))
  (def dispatch (.get mailboxes submit))
  (def (collect-results maximum wait?)
    (def events ((.ref mailboxes (if wait? 'wait 'poll)) maximum))
    (def replies (list->vector (filter deck-reply? (vector->list events))))
    (def notices (filter deck-notice? (vector->list events)))
    (def encoded-notices (make-u64vector (* 5 (length notices))))
    (let loop ((notices notices) (offset 0))
      (unless (null? notices)
        (let (notice (car notices))
        (when (= (deck-notice-kind notice) 1)
          ((.get (or (hash-get modules (deck-notice-module notice))
                     (error "timer belongs to a retired Module")) collected)
            (deck-notice-id notice)))
        (for-each (lambda (value index) (u64vector-set! encoded-notices (+ offset index) value))
          (list (deck-notice-kind notice) (deck-notice-module notice) (deck-notice-id notice)
                (deck-notice-due notice) (deck-notice-observed notice)) '(0 1 2 3 4))
        (loop (cdr notices) (+ offset 5)))))
    (def count (vector-length replies))
    (def ids (make-u64vector count))
    (def status (make-u8vector count))
    (def scopes [])
    (let loop ((index 0))
      (when (< index count)
        (let (reply (vector-ref replies index))
          (u64vector-set! ids index (deck-reply-id reply))
          (u8vector-set! status index (if (deck-reply-failed? reply) 1 0))
          (unless (deck-reply-failed? reply)
            (set! scopes (cons (deck-reply-value reply) scopes))))
        (loop (1+ index))))
    (vector ids status
      (if (null? scopes) #u8() (encode-scopes (list->vector (reverse scopes))))
      encoded-notices))
  (.o factory: open-deck-runtime
      mailbox-factory: open-deck-mailboxes
      submit-module-batch: (lambda (module rows)
        (list->u64vector (map dispatch (map (lambda (row) (cons module row)) (vector->list rows)))))
      collect: (lambda (maximum) (collect-results maximum #t))
      poll: (lambda (maximum) (collect-results maximum #f))
      activate: activate-module! retire: retire-module!
      arm: (lambda (module id delay)
             ((.get (or (hash-get modules module) (error "Deck Module is not active")) arm) id delay))
      attach-ingress: (lambda (read-fd write-fd)
                       (when ingress (error "Deck ingress already attached"))
                       (set! ingress (open-deck-ingress read-fd write-fd (.get mailboxes notify))) #t)
      statistics: (.get mailboxes statistics)
      close: (lambda ()
        (when ingress ((.get ingress close)))
        (for-each (lambda (timer) ((.get timer close))) (hash-values modules))
        (hash-clear! modules)
        ((.get mailboxes close)))))

;; A live procedure-identity check, not a cached readiness label. The C ABI
;; imports this linked checker and validates the actual constructed runtime.
(def (check-deck-kernel! runtime)
  (unless (and (eq? (.get runtime factory) open-deck-runtime)
               (eq? (.get runtime mailbox-factory) open-deck-mailboxes))
    (error "Deck runtime did not select the linked kernel and mailbox"))
  #t)
