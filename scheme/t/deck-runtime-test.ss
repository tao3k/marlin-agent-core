;;; Compare resident Actor results with the selected Lab Context operation.
(import (only-in :clan/poo/object .get .o)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
(def published (.get marlin-agent-core.init.exports Marlin deck-runtime))
(def admit (.get marlin-agent-core.init.exports Marlin context admit-scope))
(def (request turn-index)
  (.o bundle: "bundle" organization: "org" epoch: 1
      project: "project" worktree: "worktree" source-cut: "cut"
      actor: "assistant" task: "review" destination: "panel" session: "session" turn: turn-index))
(def rows (vector (request 0) (request 1) (request 2)))
(def expected (vector-map admit rows))
(def (tracked-admit request-value)
  (def admitted (admit request-value))
  (def actor-owner (current-thread))
  (.o (:: @ admitted)
      semantic-digest:
      (begin
        (unless (eq? actor-owner (current-thread))
          (error "Scope identity escaped its Actor completion boundary"))
        (.get admitted semantic-digest))))
(def runtime ((.get published open) 2 tracked-admit))
(dynamic-wind void
  (lambda ()
    (let (results ((.get runtime evaluate-batch) rows 1))
      (unless (equal? (map (lambda (scope) (.get scope semantic-digest)) (vector->list expected))
                      (map (lambda (scope) (.get scope semantic-digest)) (vector->list results)))
        (error "Deck changed Context admission order or result"))))
  (.get runtime close))
(unless (with-catch (lambda (_) #t) (lambda () ((.get runtime evaluate-batch) rows 1) #f))
  (error "closed Deck runtime accepted a batch"))
