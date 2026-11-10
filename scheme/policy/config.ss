;;; Select the contract-checked Policy scope.

(import (only-in :clan/poo/mop validate)
        "./objects.ss"
        "./types.ss")

(export PolicyScope)

(def PolicyScope (validate PolicyScopeContract PolicyOperations))
