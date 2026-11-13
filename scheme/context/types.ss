;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contract for Marlin's Context publication.

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Record String Exactly Or)
        (only-in :gerbil-ascent/temporal/lens temporal-answer?)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/types AiAgenticContextAdmissionType))

(export MarlinContextModuleContract ContextCausalContract)

(def CausalExchange
  (Record kind: [(Exactly 'agentic-ai-lab.ascent-exchange.v1)]))

(def CausalAnswer (.cc Type. '.element? temporal-answer?))

(def CausalReasonRequest (Record exchange: [CausalExchange] root: [String]))
(def CausalReplayRequest (Record exchange: [CausalExchange] root: [String] answer: [CausalAnswer]))
(def CausalReplayVerdict (Or (Exactly 'valid) (Exactly 'invalid)))

(def ContextProjectionRows (.cc Type. '.element? vector?))
(def ContextProjectionBytes (.cc Type. '.element? u8vector?))
(def ContextOrgGeneration
  (.cc Type. '.element? (lambda (v) (and (exact-integer? v) (<= 1 v 65534)))))
(def ContextOrgNodes (.cc Type. '.element? list?))
(def ContextOrgContractRequest
  (Record identity: [String] generation: [ContextOrgGeneration] source: [String]))
(def ContextOrgContractSource
  (Record kind: [(Exactly 'context.org-source.v1)] identity: [String]
          generation: [ContextOrgGeneration] source: [String] source-digest: [String]
          contract-identity: [String] contract-revision: [String] parser-identity: [String]
          active?: [(Exactly #t)] nodes: [ContextOrgNodes] semantic-digest: [String]))
(def ContextOrgSourceGraph
  (Record kind: [(Exactly 'marlin.context.org-source-graph.v1)] source: [String] graph: [Object]))
(def ContextBridgeContract
  (Record decode-batch: [(Fun ContextProjectionRows <- ContextProjectionRows)]
          encode-scopes: [(Fun ContextProjectionBytes <- ContextProjectionRows)]))

(def MarlinContextModuleContract
  (.cc (Record admit-scope: [AiAgenticContextAdmissionType] bridge: [ContextBridgeContract]
               org-contract-source: [(Fun ContextOrgContractSource <- ContextOrgContractRequest)]
               org-source-graph: [(Fun ContextOrgSourceGraph <- String)])
       identity: 'marlin/context))

(def ContextCausalContract
  (.cc (Record reason-causally: [(Fun CausalAnswer <- CausalReasonRequest)]
               replay-causal-answer: [(Fun CausalReplayVerdict <- CausalReplayRequest)])
       identity: 'marlin/context/causal))
