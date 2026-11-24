;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Import Lab's native declarations for use inside Marlin's selected modules.
(import (only-in :clan/poo/object .get .o)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/interface Context)
        (only-in :tao3k/agentic-ai-lab/modules/funflow/interface Funflow)
        (only-in :tao3k/agentic-ai-lab/modules/funflow/durable-dag
                 FunflowNode. EffectClass. PureCAS. VerifiedEpistemic. FencedEffect.
                 WorkspaceTopology.
                 OrchestratorStrategy. ParallelFrontierStrategy.
                 TieredModelStrategy. DurableGovernedStrategy.)
        (only-in :tao3k/agentic-ai-lab/modules/durable/orchestrator
                 DurableTaskPolicy. RepoNode. WorktreeNode. TaskNode. SubTaskNode.
                 DetourNode. BlockerDetourNode. SideQuestDetourNode.
                 DetourExitState. CompletedExit. ParkedExit. CancelledExit. PromotedToTaskExit.
                 AnchorKind. MainlineStepAnchor. InvariantGateAnchor.
                 DurableOrchestratorExports)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/interface TemporalCausality)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/runtime/durable-journal
                 journal-open journal-close! journal-receive! journal-begin! journal-commit!
                 journal-put! journal-get journal-rows journal-verify! journal-pack journal-hash)
        (only-in :orgize/languages/org/modules/org-contract/config OrgDirectoryProfile.)
        (only-in :orgize/languages/org/modules/org-contract/directory-runtime org-directory-validate)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/gitops/interface GitOps)
                   (GitOps LabGitOps))
        (only-in :tao3k/agentic-ai-lab/modules/module-authoring/interface
                 yuanxu-module-interface agentic-ai-lab-modules
                 agentic-ai-lab-module-closure agentic-ai-lab-module-names
                 agentic-ai-lab-module-option-schemas
                 agentic-ai-lab-module-validation-receipts))
(export LabContext LabFunflow LabDurableOrchestrator
        LabTemporal LabGitOps LabDurableJournal LabOrgDirectory
        FunflowNode. EffectClass. PureCAS. VerifiedEpistemic. FencedEffect.
        WorkspaceTopology.
        DurableTaskPolicy. RepoNode. WorktreeNode. TaskNode. SubTaskNode.
        DetourNode. BlockerDetourNode. SideQuestDetourNode.
        DetourExitState. CompletedExit. ParkedExit. CancelledExit. PromotedToTaskExit.
        AnchorKind. MainlineStepAnchor. InvariantGateAnchor.
        OrchestratorStrategy. ParallelFrontierStrategy.
        TieredModelStrategy. DurableGovernedStrategy.
        yuanxu-module-interface agentic-ai-lab-modules
        agentic-ai-lab-module-closure agentic-ai-lab-module-names
        agentic-ai-lab-module-option-schemas
        agentic-ai-lab-module-validation-receipts)

(def LabContext (.get Context core exports))
(def LabFunflow (.get Funflow core exports))
(def LabDurableOrchestrator DurableOrchestratorExports)
(def LabTemporal (.get TemporalCausality core exports))

(def LabDurableJournal
  (.o open: journal-open
      close!: journal-close!
      receive!: journal-receive!
      begin!: journal-begin!
      commit!: journal-commit!
      put!: journal-put!
      get: journal-get
      rows: journal-rows
      verify!: journal-verify!
      pack: journal-pack
      hash: journal-hash))

(def LabOrgDirectory
  (.o profile-prototype: OrgDirectoryProfile.
      validate: org-directory-validate))
