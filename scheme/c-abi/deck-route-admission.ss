;;; Rooted-object entry for the Deck model-route admission path.

(import :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .get .o)
        (only-in "../deck-runtime/config.ss" DeckScope)
        (only-in "../policy/routing.ss" make-marlin-deck-runtime-model-route-policy)
        (only-in "../policy/strategy.ss"
                 make-marlin-deck-runtime-strategy-facts make-marlin-deck-runtime-strategy-rule))

(export main)

(def (main . _) (void))

(def deck #f)

(def (open worker-count)
  (when deck (error "Deck route admission is already open"))
  (set! deck ((.get DeckScope open) worker-count))
  #t)

(def (evaluate input)
  (unless deck (error "Deck route admission is closed"))
  (def command-text (vector-ref input 0))
  (def scope-label (vector-ref input 1))
  (def facts-value (make-marlin-deck-runtime-strategy-facts #f '() '() '() scope-label))
  (def (turn candidate)
    (def name (vector-ref candidate 0))
    (def policy
      (make-marlin-deck-runtime-model-route-policy
       name (vector-ref candidate 1) (vector-ref candidate 2)
       (vector->list (vector-ref candidate 3)) (vector->list (vector-ref candidate 4))
       (vector-ref candidate 5) (vector-ref candidate 6)))
    (def rule (make-marlin-deck-runtime-strategy-rule name name #f '() '() '() #f "allow" #f))
    (.o policies: (list policy) rules: (list rule) facts: facts-value
        command: command-text agent-scope: scope-label))
  (def decisions ((.get deck evaluate-batch) (vector-map turn (vector-ref input 2)) 1))
  (vector-map (lambda (decision) (.get decision matched)) decisions))

(def (close _)
  (when deck
    ((.get deck close))
    (set! deck #f))
  #t)

(begin-foreign
  (c-define (deck-open root) (int64) int64 "marlin_deck_route_open" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-route-admission#open))
  (c-define (deck-call root) (int64) int64 "marlin_deck_route_evaluate" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-route-admission#evaluate))
  (c-define (deck-close root) (int64) int64 "marlin_deck_route_close" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-route-admission#close)))
