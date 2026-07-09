;;; -*- Gerbil -*-
;;; Engineering note: The profile catalog is a declarative index over compiler
;;; receipts. Mainline ordering is data-driven because adding a profile should
;;; update catalog data, not duplicate manual vector-ref scaffolding.
package: config-interface/modules

(import (only-in :clan/poo/object .get)
        :config-interface/modules/kinds
        :config-interface/modules/policy-pack-support
        :config-interface/modules/policy-pack-real-repair
        :config-interface/modules/policy-pack-real-policy-001
        :config-interface/modules/policy-pack-real-policy-basic
        :config-interface/modules/policy-pack-failure-combination)

(def +marlin-loop-vertical-mainline-descriptor-indexes+
  '(0 3 5 6 7 8 2))

(export marlinLoopPolicyProfileProjectionDescriptor
        marlinLoopPolicyProfileProjectionDescriptors
        marlinLoopVerticalMainlineProjectionDescriptors
        marlinLoopPolicyProjectionModuleFromDescriptor
        marlinLoopPolicyProjectionModules
        marlinLoopPolicyProfileCompilerReceipts)

;;; Boundary: POO Flow loop profiles are exported as projection modules.
;; : (-> MarlinInput MarlinResult)
(def (marlinLoopPolicyProjectionModule module-id-value
                                       profile-id-value
                                       poo-flow-module-value
                                       capability-lanes-value
                                       vertical-case-id-value
                                       vertical-capability-tags-value
                                       compiler-receipt-value)
  (marlin-policy-object<-alist
   (list
    (cons 'kind "marlin.config-interface.loop-policy.profile-projection-module.v1")
    (cons 'module-id module-id-value)
    (cons 'profile-id profile-id-value)
    (cons 'owner "gerbil-poo-flow")
    (cons 'source-module ":config-interface/modules/policy-pack")
    (cons 'poo-flow-module poo-flow-module-value)
    (cons 'poo-flow-capability-lanes capability-lanes-value)
    (cons 'vertical-case-id vertical-case-id-value)
    (cons 'vertical-capability-tags vertical-capability-tags-value)
    (cons 'vertical-mainline? (if vertical-case-id-value #t #f))
    (cons 'rust-type marlin-poo-loop-program-compiler-receipt-kind)
    (cons 'scheme-boundary "scheme-types-to-rust-types")
    (cons 'serialization-boundary "rust-owned-cli-trace-cross-process")
    (cons 'compiler-receipt compiler-receipt-value))))

;;; Boundary: Module descriptors are the Scheme-owned profile catalog.
;; marlinLoopPolicyProfileProjectionDescriptor
;;   : (-> ModuleId ProfileId PooFlowModule CapabilityLaneVector
;;         VerticalCaseId CapabilityTagVector CompilerReceipt
;;         ProjectionDescriptor)
(def (marlinLoopPolicyProfileProjectionDescriptor module-id-value
                                                  profile-id-value
                                                  poo-flow-module-value
                                                  capability-lanes-value
                                                  vertical-case-id-value
                                                  vertical-capability-tags-value
                                                  compiler-receipt-value)
  (marlin-policy-object<-alist
   (list
    (cons 'module-id module-id-value)
    (cons 'profile-id profile-id-value)
    (cons 'poo-flow-module poo-flow-module-value)
    (cons 'poo-flow-capability-lanes capability-lanes-value)
    (cons 'vertical-case-id vertical-case-id-value)
    (cons 'vertical-capability-tags vertical-capability-tags-value)
    (cons 'vertical-mainline? (if vertical-case-id-value #t #f))
    (cons 'compiler-receipt compiler-receipt-value))))

;;; Boundary: Descriptor-to-module mapping is the only Rust projection shape.
;; MarlinResult <- MarlinInput
(def (marlinLoopPolicyProjectionModuleFromDescriptor descriptor-value)
  (marlinLoopPolicyProjectionModule
   (.get descriptor-value module-id)
   (.get descriptor-value profile-id)
   (.get descriptor-value poo-flow-module)
   (.get descriptor-value poo-flow-capability-lanes)
   (.get descriptor-value vertical-case-id)
   (.get descriptor-value vertical-capability-tags)
   (.get descriptor-value compiler-receipt)))

;;; Boundary: Rust consumes compiler receipts through the module projection catalog.
;;; The fixed vector projection preserves descriptor index identity for trace receipts.
;; marlinLoopPolicyProjectionModuleReceipts
;;   : (-> ProjectionModuleVector CompilerReceiptVector)
;;   | doc m%
;;       `marlinLoopPolicyProjectionModuleReceipts modules` extracts compiler
;;       receipts while preserving the module descriptor order expected by Rust.
;;
;;       # Examples
;;       ```scheme
;;       (vector-length (marlinLoopPolicyProjectionModuleReceipts modules))
;;       ;; => (vector-length modules)
;;       ```
;;     %
(def (marlinLoopPolicyProjectionModuleReceipts modules)
  (marlin-vector-map
   (lambda (module-value)
     (.get module-value compiler-receipt))
   modules))

;;; Boundary: Public profile compiler descriptors are the Scheme extension surface.
;; : (-> MarlinInput MarlinResult)
(def (marlinLoopPolicyProfileProjectionDescriptors)
  (vector
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-repair-001"
    "real-repair-001/reactive-tool-loop"
    "loop-engine"
    (vector "fun-flow" "loop-engine" "sandbox" "tool-handoff")
    "real-repair-001"
    (vector '+scripted-e2e '+tool-repair '+verification)
    (marlinRealRepair001LoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.failure-retry"
    "marlin-failure-retry-profile/typed-recovery"
    "loop-engine"
    (vector "fun-flow" "loop-engine" "retry")
    #f
    (vector)
    (marlinFailureRetryLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.policy-combination-matrix"
    "policy-combination/memory-rewrite-checker"
    "loop-engine"
    (vector "fun-flow" "loop-engine" "memory" "rewrite" "checker")
    "policy-combination/memory-rewrite-checker"
    (vector '+policy-combination '+memory '+rewrite '+checker)
    (marlinPolicyCombinationMatrixLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-policy-001-sandbox-denylist"
    "real-policy-001/sandbox-denylist"
    "loop-engine"
    (vector "loop-engine" "sandbox" "denylist")
    "real-policy-001/sandbox-denylist"
    (vector '+sandbox '+denylist)
    (marlinRealPolicy001SandboxDenylistLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-policy-001-tool-sandbox"
    "real-policy-001/tool-sandbox"
    "loop-engine"
    (vector "loop-engine" "sandbox" "tool-handoff")
    #f
    (vector)
    (marlinRealToolSandboxLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-policy-002-retry-budget"
    "real-policy-002/retry-budget"
    "loop-engine"
    (vector "loop-engine" "retry" "tool-handoff")
    "real-policy-002/retry-budget"
    (vector '+retry-budget '+failure-policy)
    (marlinRealPolicy002RetryBudgetLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-policy-003-maker-checker"
    "real-policy-003/maker-checker"
    "loop-engine"
    (vector "loop-engine" "maker" "checker" "model" "verification")
    "real-policy-003/maker-checker"
    (vector '+maker '+checker)
    (marlinRealPolicy003MakerCheckerLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-policy-004-dynamic-rewrite"
    "real-policy-004/dynamic-rewrite"
    "loop-engine"
    (vector "loop-engine" "rewrite" "tool-handoff" "verification")
    "real-policy-004/dynamic-rewrite"
    (vector '+dynamic-rewrite '+repair)
    (marlinRealPolicy004DynamicRewriteLoopProgramCompilerReceipt))
   (marlinLoopPolicyProfileProjectionDescriptor
    "poo-flow.loop-engine.real-policy-005-memory-recall"
    "real-policy-005/memory-recall"
    "loop-engine"
    (vector "loop-engine" "memory" "tool-handoff")
    "real-policy-005/memory-recall"
    (vector '+memory-recall '+tool-selection)
    (marlinRealPolicy005MemoryRecallLoopProgramCompilerReceipt))))

;;; Boundary: Vertical mainline case order lives with the Scheme profile catalog.
;; marlinLoopVerticalMainlineProjectionDescriptors
;;   : (-> ProjectionDescriptorVector)
(def (marlinLoopVerticalMainlineProjectionDescriptors)
  (let (descriptors (marlinLoopPolicyProfileProjectionDescriptors))
    (list->vector
     (map (lambda (descriptor-index)
            (vector-ref descriptors descriptor-index))
          +marlin-loop-vertical-mainline-descriptor-indexes+))))

;;; Boundary: Public profile compiler projections are module-derived.
;; MarlinResult <- MarlinInput
(def (marlinLoopPolicyProjectionModules)
  (marlin-vector-map
   marlinLoopPolicyProjectionModuleFromDescriptor
   (marlinLoopPolicyProfileProjectionDescriptors)))

;;; Boundary: Public profile compiler receipts remain a Rust-facing projection view.
;; MarlinResult <- MarlinInput
(def (marlinLoopPolicyProfileCompilerReceipts)
  (marlinLoopPolicyProjectionModuleReceipts
   (marlinLoopPolicyProjectionModules)))
