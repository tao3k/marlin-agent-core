;;; Session scope exported to Marlin's policy consumers.

(import (only-in :clan/poo/object .cc .o)
        (only-in :clan/poo/object-contract ObjectContract.)
        (only-in :clan/poo/type String))

(export SessionScopeContract)

(def SessionScopeContract
  (.cc ObjectContract.
       responsibilities: (.o sharing: String isolation: String snapshot: String)))
