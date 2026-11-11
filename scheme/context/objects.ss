;;; Assemble Marlin's Context operations.

(import (only-in :clan/poo/object .o)
        "./func.ss"
        "./projection.ss"
        "./evidence.ss")

(export MarlinContextOperations EnableCausalEvidence)

(def MarlinContextOperations
  (.o admit-scope: marlin-context-admit-scope
      bridge: (.o decode-batch: marlin-context-batch-requests
                  encode-scopes: marlin-context-scopes->bytes)))

(def (EnableCausalEvidence inherited)
  (.o (:: self inherited)
      reason-causally: context-reason-causally
      replay-causal-answer: context-replay-causal-answer))
