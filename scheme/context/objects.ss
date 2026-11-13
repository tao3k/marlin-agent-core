;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Assemble Marlin's Context operations.

(import (only-in :clan/poo/object .o)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/funs-scope
                 agentic-ai-lab-ai-agentic-context-scope-admit)
        :marlin-agent-core/context/projection
        :marlin-agent-core/context/org-contract
        :marlin-agent-core/context/org-source
        :marlin-agent-core/context/evidence)

(export MarlinContextOperations EnableCausalEvidence)

(def MarlinContextOperations
  (.o admit-scope: agentic-ai-lab-ai-agentic-context-scope-admit
      org-contract-source: marlin-context-org-contract-source
      org-source-graph: marlin-context-org-source-graph
      bridge: (.o decode-batch: marlin-context-batch-requests
                  encode-scopes: marlin-context-scopes->bytes)))

(def (EnableCausalEvidence inherited)
  (.o (:: self inherited)
      reason-causally: context-reason-causally
      replay-causal-answer: context-replay-causal-answer))
