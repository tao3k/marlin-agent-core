;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contract for Marlin's Context publication.

(import (only-in :marlin-agent-core/interface LabContext)
        (only-in :clan/poo/object .get .cc)

        (only-in :clan/poo/type Fun Object Type.)
        (only-in :clan/poo/type Record String Exactly Or)
        (only-in :gerbil-ascent/temporal/lens temporal-answer?))

(export MarlinContextSystemContract ContextCausalContract ContextDurableContract)

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

(def ContextProgressiveContract
  (Record element-graph: [(Fun Object <- Object)]
          select-elements: [(Fun Object <- Object)]
          dispatch: [(Fun Object <- Object)]
          repair-org: [(Fun Object <- Object)]
          babel-upsert: [(Fun String <- Object)]
          checklist-set: [(Fun String <- Object)]
          headline-set-todo: [(Fun String <- Object)]
          fission-subtree: [(Fun Object <- Object)]))

(def ContextOrchestratorContract
  (Record derive-worktree: [(Fun Object <- Object)]
          derive-task: [(Fun Object <- Object)]
          orchestrate-task: [(Fun Object <- Object)]
          replan-epoch: [(Fun Object <- Object)]
          admit-node-effect: [(Fun Object <- Object)]
          envelope-meet: [(Fun Object <- Object)]
          invalidate-premises: [(Fun Object <- Object)]
          workspace-plan: [(Fun Object <- Object)]
          apply-org-overlay: [(Fun Object <- Object)]
          reanchor-mainline: [(Fun Object <- Object)]
          reconcile-detour-exit: [(Fun Object <- Object)]
          perceive-claims: [(Fun ContextOrgNodes <- Object)]
          assert-write-owner: [(Fun Object <- Object)]
          claim-node!: [(Fun Object <- Object)]
          extend-node!: [(Fun Object <- Object)]
          handover-node!: [(Fun Object <- Object)]
          record-cas!: [(Fun Object <- Object)]
          recover-task: [(Fun Object <- Object)]
          commit-auto-archive!: [(Fun Object <- Object)]
          reopen-task!: [(Fun Object <- Object)]
          project-workbench-tabs: [(Fun Object <- Object)]))

(def MarlinContextSystemContract
  (.cc (Record admit-scope: [(.get LabContext admission-type)] bridge: [ContextBridgeContract]
               org-contract-source: [(Fun ContextOrgContractSource <- ContextOrgContractRequest)]
               org-source-graph: [(Fun ContextOrgSourceGraph <- String)]
               progressive: [ContextProgressiveContract])
       identity: 'marlin/context))

(def ContextCausalContract
  (.cc (Record reason-causally: [(Fun CausalAnswer <- CausalReasonRequest)]
               replay-causal-answer: [(Fun CausalReplayVerdict <- CausalReplayRequest)])
       identity: 'marlin/context/causal))

(def ContextDurableContract
  (.cc (Record durable: [ContextOrchestratorContract])
       identity: 'marlin/context/durable))
