;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Assemble Marlin's Context operations.

(import (only-in :marlin-agent-core/interface LabContext)
        (only-in :clan/poo/object .get .o)

        :marlin-agent-core/context-system/projection
        :marlin-agent-core/context-system/org-contract
        :marlin-agent-core/context-system/org-source
        :marlin-agent-core/context-system/evidence)

(export MarlinContextSystemOperations EnableCausalEvidence)

(def MarlinContextSystemOperations
  (.o admit-scope: (.get LabContext admit-scope)
      org-contract-source: marlin-context-org-contract-source
      org-source-graph: marlin-context-org-source-graph
      bridge: (.o decode-batch: marlin-context-batch-requests
                  encode-scopes: marlin-context-scopes->bytes)))

(def (EnableCausalEvidence inherited)
  (.o (:: self inherited)
      reason-causally: context-reason-causally
      replay-causal-answer: context-replay-causal-answer))
