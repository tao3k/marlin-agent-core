;;; Selected session policy scope.

(import (only-in :clan/poo/mop validate)
        "objects.ss"
        "types.ss")

(export SessionScope)

(def SessionScope (validate SessionScopeContract SessionDefaults))
