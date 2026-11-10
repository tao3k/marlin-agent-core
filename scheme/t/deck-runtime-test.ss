;;; Qualify the published Deck Runtime with real resident actors.

(import (only-in :clan/poo/object .get .o)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in "../policy/routing.ss" make-marlin-deck-runtime-model-route-policy)
        (only-in "../policy/strategy.ss"
                 make-marlin-deck-runtime-strategy-facts make-marlin-deck-runtime-strategy-rule))

(def deck-runtime (.get marlin-agent-core.init.exports Marlin deck-runtime))
(def metadata (.get deck-runtime metadata))
(unless (and (equal? (.get metadata module-id) "deck-runtime") (positive? (.get metadata version)))
  (error "invalid Deck Runtime metadata" metadata))

(def route (make-marlin-deck-runtime-model-route-policy
            "review" "provider" "model" '("review") '("local") "shared" "workspace"))
(def rule (make-marlin-deck-runtime-strategy-rule "review-rule" "review" #f '() '() '() #f "allow" #f))
(def scope-facts (make-marlin-deck-runtime-strategy-facts "session" '() '() '() "local"))
(def (turn request-command)
  (.o policies: (list route) rules: (list rule) facts: scope-facts
      command: request-command agent-scope: "local"))

(let* ((runtime ((.get deck-runtime open) 2))
       (evaluate (.get runtime evaluate-batch))
       (close (.get runtime close))
       (batch (vector (turn "review first") (turn "unmatched") (turn "review third"))))
  (dynamic-wind
    void
    (lambda ()
      (let (results (evaluate batch 1))
        (unless (and (equal? (map (lambda (result) (.get result command)) (vector->list results))
                             '("review first" "unmatched" "review third"))
                     (equal? (map (lambda (result) (.get result matched)) (vector->list results))
                             '(#t #f #t)))
          (error "Deck actors changed batch order or policy results" results))))
    close)
  (unless (with-catch (lambda (_) #t) (lambda () (evaluate batch 1) #f))
    (error "closed Deck Runtime accepted another batch")))
