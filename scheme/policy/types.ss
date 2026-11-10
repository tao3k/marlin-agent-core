;;; Contract for the callable policy scope.

(import (only-in :clan/poo/object .cc .o)
        (only-in :clan/poo/mop Type.)
        (only-in :clan/poo/object-contract ObjectContract.))

(export PolicyScopeContract)

(def PolicyOperation (.cc Type. '.element? procedure?))

(def PolicyScopeContract
  (.cc ObjectContract.
       responsibilities: (.o select-model-route: PolicyOperation
                             project-routing: PolicyOperation
                             compile-loop-graph: PolicyOperation
                             next-continuation: PolicyOperation)))
