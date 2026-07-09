;;; -*- Gerbil -*-
;;; Boundary: Facade re-exports prefab policy pack modules.

package: config-interface/modules

(import :config-interface/modules/policy-pack-core
        :config-interface/modules/policy-pack-presentation
        :config-interface/modules/policy-pack-slot-merge
        :config-interface/modules/policy-pack-profile-catalog
        :config-interface/modules/policy-pack-real-repair
        :config-interface/modules/policy-pack-real-policy-001
        :config-interface/modules/policy-pack-real-policy-basic
        :config-interface/modules/policy-pack-failure-combination
        :config-interface/modules/policy-pack-receipts)

(export marlinPolicyPack
        defmarlin-policy-pack
        marlinDefaultPolicyPack
        marlinPackCatalog
        marlin-pack-catalog-find
        marlin-pack-catalog-root
        marlinPackCatalogPresentation
        marlinPolicyPackInventory
        marlinPolicyPackPresentation
        marlinPolicyProjection
        marlinPooLoopProgramCompilerReceipt
        marlin-policy-slot-merge-receipt-kind
        marlin-policy-slot-merge-algebra-demo-receipt-kind
        marlinPolicySlotMergeReceipt
        marlinPolicySlotMergeUnion
        marlinPolicySlotMergeIntersection
        marlinPolicySlotMergeMin
        marlinPolicySlotMergeOrderedAppend
        marlinPolicySlotMergeConflictError
        marlinPolicySlotMergeAuditReceipt
        marlinPolicySlotMergeAuditReceipts
        marlinPolicySlotMergeForcedSlots
        marlinPolicySlotMergeAlgebraDemoReceipt
        marlinLoopPolicyProfileProjectionDescriptor
        marlinLoopPolicyProfileProjectionDescriptors
        marlinLoopVerticalMainlineProjectionDescriptors
        marlinLoopPolicyProjectionModuleFromDescriptor
        marlinLoopPolicyProjectionModules
        marlinLoopPolicyProfileCompilerReceipts
        marlinRealRepair001SlotMergeAlgebraReceipts
        marlinRealRepair001ResolvedPolicyPack
        marlinRealRepair001LoopProgram
        marlinRealRepair001LoopProgramCompilerReceipt
        marlinRealPolicy001SandboxDenylistSlotMergeAlgebraReceipts
        marlinRealPolicy001SandboxDenylistResolvedPolicyPack
        marlinRealPolicy001SandboxDenylistLoopProgram
        marlinRealPolicy001SandboxDenylistLoopProgramCompilerReceipt
        marlinRealToolSandboxSlotMergeAlgebraReceipts
        marlinRealToolSandboxResolvedPolicyPack
        marlinRealToolSandboxLoopProgram
        marlinRealToolSandboxLoopProgramCompilerReceipt
        marlinRealPolicy002RetryBudgetSlotMergeAlgebraReceipts
        marlinRealPolicy002RetryBudgetResolvedPolicyPack
        marlinRealPolicy002RetryBudgetLoopProgram
        marlinRealPolicy002RetryBudgetLoopProgramCompilerReceipt
        marlinRealPolicy003MakerCheckerSlotMergeAlgebraReceipts
        marlinRealPolicy003MakerCheckerResolvedPolicyPack
        marlinRealPolicy003MakerCheckerLoopProgram
        marlinRealPolicy003MakerCheckerLoopProgramCompilerReceipt
        marlinRealPolicy004DynamicRewriteSlotMergeAlgebraReceipts
        marlinRealPolicy004DynamicRewriteResolvedPolicyPack
        marlinRealPolicy004DynamicRewriteLoopProgram
        marlinRealPolicy004DynamicRewriteLoopProgramCompilerReceipt
        marlinRealPolicy005MemoryRecallSlotMergeAlgebraReceipts
        marlinRealPolicy005MemoryRecallResolvedPolicyPack
        marlinRealPolicy005MemoryRecallLoopProgram
        marlinRealPolicy005MemoryRecallLoopProgramCompilerReceipt
        marlinFailureRetrySlotMergeAlgebraReceipts
        marlinFailureRetryResolvedPolicyPack
        marlinFailureRetryLoopProgram
        marlinFailureRetryLoopProgramCompilerReceipt
        marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts
        marlinPolicyCombinationMatrixResolvedPolicyPack
        marlinPolicyCombinationMatrixLoopProgram
        marlinPolicyCombinationMatrixLoopProgramCompilerReceipt
        marlinPolicyModuleEvaluationReceipt
        marlinPolicyBudgetReceipt
        marlinPolicyCatalogResolutionReceipt
        marlinPolicyProjectionReceipts
        marlinPolicyProjectionChainReceipt)
