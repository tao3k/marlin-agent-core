;;; Fixed-owner Deck actors use the bridge's resident mailbox service.

(import (only-in :clan/poo/object .get .o)
        (only-in :clan/poo/mop validate)
        (only-in :gerbil-scheme-rust/scheme/actor-batch
                 make-gerbil-rs-actor-batch gerbil-rs-actor-batch! gerbil-rs-actor-batch-close!)
        (only-in "../policy/strategy-evaluation.ss" marlin-deck-runtime-strategy-selection)
        (only-in "./types.ss" DeckTurn))

(export open-deck-runtime)

(def (evaluate-turn _worker row)
  (def turn (validate DeckTurn row))
  (marlin-deck-runtime-strategy-selection
   (.get turn policies) (.get turn rules) (.get turn facts)
   (.get turn command) (.get turn agent-scope)))

(def (open-deck-runtime workers)
  (def owner (current-thread))
  (def actors (make-gerbil-rs-actor-batch evaluate-turn workers))
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
