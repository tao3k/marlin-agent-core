;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Assemble Marlin's Context operations.

(import (only-in :marlin-agent-core/interface LabContext)
        (only-in :clan/poo/object .get .o)

        :marlin-agent-core/context-system/projection
        :marlin-agent-core/context-system/org-contract
        :marlin-agent-core/context-system/org-source
        :marlin-agent-core/context-system/progressive
        :marlin-agent-core/context-system/orchestrator
        :marlin-agent-core/context-system/evidence)

(export MarlinContextSystemOperations EnableCausalEvidence)

(def MarlinContextSystemOperations
  (.o admit-scope: (.get LabContext admit-scope)
      org-contract-source: marlin-context-org-contract-source
      org-source-graph: marlin-context-org-source-graph
      progressive: (.o element-graph: marlin-context-element-graph
                       select-elements: marlin-context-select-elements
                       dispatch: marlin-context-progressive-dispatch
                       repair-org: marlin-context-repair-org
                       babel-upsert: marlin-context-babel-upsert
                       checklist-set: marlin-context-checklist-set
                       headline-set-todo: marlin-context-headline-set-todo
                       fission-subtree: marlin-context-fission-subtree)
      orchestrator: (.o derive-worktree: marlin-derive-worktree
                        derive-task: marlin-derive-task
                        orchestrate-task: marlin-orchestrate-task
                        replan-epoch: marlin-replan-task-epoch
                        admit-node-effect: marlin-admit-node-effect
                        envelope-meet: marlin-envelope-meet
                        invalidate-premises: marlin-invalidate-premises
                        workspace-plan: marlin-workspace-plan
                        apply-org-overlay: marlin-apply-org-overlay
                        reanchor-mainline: marlin-reanchor-mainline
                        reconcile-detour-exit: marlin-reconcile-detour-exit
                        perceive-claims: marlin-perceive-claims
                        assert-write-owner: marlin-assert-write-owner
                        claim-node!: marlin-claim-node!
                        extend-node!: marlin-extend-node!
                        handover-node!: marlin-handover-node!
                        record-cas!: marlin-record-task-cas!
                        recover-task: marlin-recover-task
                        commit-auto-archive!: marlin-commit-auto-archive!
                        reopen-task!: marlin-reopen-task!
                        project-workbench-tabs: marlin-project-workbench-tabs)
      bridge: (.o decode-batch: marlin-context-batch-requests
                  encode-scopes: marlin-context-scopes->bytes)))

(def (EnableCausalEvidence inherited)
  (.o (:: self inherited)
      reason-causally: context-reason-causally
      replay-causal-answer: context-replay-causal-answer))
