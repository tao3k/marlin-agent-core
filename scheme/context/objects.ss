;;; Assemble Marlin's Context operations.

(import (only-in :clan/poo/object .o)
        "./func.ss"
        "./evidence.ss")

(export MarlinContextOperations EnableCausalEvidence)

(def MarlinContextOperations
  (.o admit-scope: marlin-context-admit-scope))

(def (EnableCausalEvidence inherited)
  (.o (:: self inherited)
      reason-causally: context-reason-causally
      replay-causal-answer: context-replay-causal-answer))
