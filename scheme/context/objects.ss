;;; Assemble Marlin's Context operations.

(import (only-in :clan/poo/object .o)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/funs-scope
                 agentic-ai-lab-ai-agentic-context-scope-admit)
        "./projection.ss"
        "./evidence.ss")

(export MarlinContextOperations EnableCausalEvidence)

(def MarlinContextOperations
  (.o admit-scope: agentic-ai-lab-ai-agentic-context-scope-admit
      bridge: (.o decode-batch: marlin-context-batch-requests
                  encode-scopes: marlin-context-scopes->bytes)))

(def (EnableCausalEvidence inherited)
  (.o (:: self inherited)
      reason-causally: context-reason-causally
      replay-causal-answer: context-replay-causal-answer))
