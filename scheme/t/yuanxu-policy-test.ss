;;; Exercise the selected package entry and its session policy publication.

(import (only-in :clan/poo/object .get)
        "../init.ss")

(let (session (.get marlin-agent-core.init.exports Marlin session))
  (unless (and (equal? (.get session sharing) "shared")
               (equal? (.get session isolation) "branch-isolated")
               (equal? (.get session snapshot) "enabled"))
    (error "unexpected Marlin session policy" session)))
