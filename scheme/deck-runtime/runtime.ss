;;; Context identities route messages to resident Gerbil mailbox Actors.

(import (only-in :clan/poo/object .get .ref .o)
        "./mailbox.ss")

(export open-deck-runtime)

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
  (def mailboxes (open-deck-mailboxes context-actor-key admit-message))
  (def dispatch (.get mailboxes submit))
  (def (collect-results maximum)
    (def replies ((.get mailboxes collect) maximum))
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
      (if (null? scopes) #u8() (encode-scopes (list->vector (reverse scopes))))))
  (.o submit-batch: (lambda (rows) (list->u64vector (map dispatch (vector->list rows))))
      collect: collect-results
      statistics: (.get mailboxes statistics)
      close: (.get mailboxes close)))
