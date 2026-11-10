;;; Exercise both publications through the selected package entry.

(import (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(let (context (.get marlin-agent-core.init.exports Lab ai-agentic-context))
  (unless (equal? (.get context profile feature-ids) '(ai-agentic-context/core))
    (error "expected Lab's core Context profile" context)))

(let (policy (.get marlin-agent-core.init.exports Marlin policy))
  (unless (eq? ((.get policy select-model-route) '() '() #f "test" "local") #f)
    (error "empty policy must not select a model route")))
