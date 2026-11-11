;;; The published Context operation runs on resident SDK green actors.

(import (only-in :clan/poo/object .get .o)
        (only-in :gerbil-scheme-rust/scheme/actor-batch
                 make-gerbil-rs-actor-batch gerbil-rs-actor-batch! gerbil-rs-actor-batch-close!))

(export open-deck-runtime)

(def (open-deck-runtime workers admit-context)
  (def owner (current-thread))
  (def (admit-turn _worker request)
    (def admitted (admit-context request))
    ;; A Scope identity is a lazy POO slot. Complete it before the actor ack,
    ;; so the cooperative quantum covers the full admission operation.
    (unless (string? (.get admitted semantic-digest))
      (error "Context admission did not produce a Scope identity"))
    admitted)
  (def actors (make-gerbil-rs-actor-batch admit-turn workers))
  (def (assert-owner)
    (unless (eq? owner (current-thread))
      (error "Deck runtime belongs to its opening Scheme thread")))
  (.o evaluate-batch: (lambda (rows quantum)
                        (assert-owner)
                        (gerbil-rs-actor-batch! actors (vector (s64vector quantum) rows)))
      close: (lambda ()
               (assert-owner)
               (gerbil-rs-actor-batch-close! actors)
               #t)))
