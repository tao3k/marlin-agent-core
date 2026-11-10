;;; Exercise both publications through the selected package entry.

(import (only-in :clan/poo/object .get)
        "../init.ss")

(let (session (.get marlin-agent-core.init.exports Marlin session))
  (unless (and (equal? (.get session sharing) "shared")
               (equal? (.get session isolation) "branch-isolated")
               (equal? (.get session snapshot) "enabled"))
    (error "unexpected Marlin session policy" session)))

(let (policy (.get marlin-agent-core.init.exports Marlin policy))
  (unless (eq? ((.get policy select-model-route) '() '() #f "test" "local") #f)
    (error "empty policy must not select a model route")))
