;;; -*- Gerbil -*-
;;; Boundary: Config-interface policy pack compiler receipts execute in real gxi.

(import (only-in :clan/poo/object .get)
        :config-interface/lib)

;;; Boundary: Local scalar assertions avoid extra std/test runner behavior.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

(def failure-retry-compiler-receipt
  (marlinFailureRetryLoopProgramCompilerReceipt))

(def failure-retry-policy-pack
  (.get failure-retry-compiler-receipt resolved-policy-pack))

(def failure-retry-hot-pack
  (.get failure-retry-policy-pack hot))

(def failure-retry-budget-caps
  (.get failure-retry-hot-pack budget_caps))

(def failure-retry-continuation-table
  (.get failure-retry-hot-pack continuation_table))

(def failure-retry-loop-program
  (.get failure-retry-compiler-receipt loop-program))

(def real-repair-compiler-receipt
  (marlinRealRepair001LoopProgramCompilerReceipt))

(def real-repair-policy-pack
  (.get real-repair-compiler-receipt resolved-policy-pack))

(def real-repair-audit-pack
  (.get real-repair-policy-pack audit))

(def real-repair-policy-mixins
  (.get real-repair-audit-pack policy_mixins))

(def real-repair-merge-receipts
  (.get real-repair-audit-pack merge_receipts))

(def real-repair-loop-program
  (.get real-repair-compiler-receipt loop-program))

(def policy-combination-compiler-receipt
  (marlinPolicyCombinationMatrixLoopProgramCompilerReceipt))

(def policy-combination-policy-pack
  (.get policy-combination-compiler-receipt resolved-policy-pack))

(def policy-combination-hot-pack
  (.get policy-combination-policy-pack hot))

(def policy-combination-audit-pack
  (.get policy-combination-policy-pack audit))

(def policy-combination-loop-program
  (.get policy-combination-compiler-receipt loop-program))

(def projection-modules
  (marlinLoopPolicyProjectionModules))

(def projection-descriptors
  (marlinLoopPolicyProfileProjectionDescriptors))

(def vertical-mainline-descriptors
  (marlinLoopVerticalMainlineProjectionDescriptors))

(def profile-compiler-receipts
  (marlinLoopPolicyProfileCompilerReceipts))

(def real-policy-003-compiler-receipt
  (vector-ref profile-compiler-receipts 6))

(def real-policy-003-policy-pack
  (.get real-policy-003-compiler-receipt resolved-policy-pack))

(def real-policy-003-hot-pack
  (.get real-policy-003-policy-pack hot))

(def real-policy-003-loop-program
  (.get real-policy-003-compiler-receipt loop-program))

(def real-policy-004-compiler-receipt
  (vector-ref profile-compiler-receipts 7))

(def real-policy-004-policy-pack
  (.get real-policy-004-compiler-receipt resolved-policy-pack))

(def real-policy-004-hot-pack
  (.get real-policy-004-policy-pack hot))

(def real-policy-004-loop-program
  (.get real-policy-004-compiler-receipt loop-program))

(def real-policy-005-compiler-receipt
  (vector-ref profile-compiler-receipts 8))

(def real-policy-005-policy-pack
  (.get real-policy-005-compiler-receipt resolved-policy-pack))

(def real-policy-005-hot-pack
  (.get real-policy-005-policy-pack hot))

(def real-policy-005-loop-program
  (.get real-policy-005-compiler-receipt loop-program))

(check (vector-length projection-descriptors) => 9)
(check (.get (vector-ref projection-descriptors 0) module-id)
       => "poo-flow.loop-engine.real-repair-001")
(check (.get (vector-ref projection-descriptors 0) poo-flow-module)
       => "loop-engine")
(check (vector-ref (.get (vector-ref projection-descriptors 0)
                         poo-flow-capability-lanes)
                   0)
       => "fun-flow")
(check (.get (.get (vector-ref projection-descriptors 2)
                   compiler-receipt)
             profile-id)
       => "policy-combination/memory-rewrite-checker")
(check (.get (vector-ref projection-descriptors 0) vertical-case-id)
       => "real-repair-001")
(check (vector-ref (.get (vector-ref projection-descriptors 0)
                         vertical-capability-tags)
                   0)
       => '+scripted-e2e)
(check (.get (vector-ref projection-descriptors 1) vertical-mainline?)
       => #f)
(check (.get (vector-ref projection-descriptors 4) vertical-mainline?)
       => #f)
(check (.get (vector-ref projection-descriptors 8) vertical-mainline?)
       => #t)

(check (vector-length vertical-mainline-descriptors) => 7)
(check (.get (vector-ref vertical-mainline-descriptors 0) profile-id)
       => "real-repair-001/reactive-tool-loop")
(check (.get (vector-ref vertical-mainline-descriptors 1) profile-id)
       => "real-policy-001/sandbox-denylist")
(check (.get (vector-ref vertical-mainline-descriptors 2) profile-id)
       => "real-policy-002/retry-budget")
(check (.get (vector-ref vertical-mainline-descriptors 3) profile-id)
       => "real-policy-003/maker-checker")
(check (.get (vector-ref vertical-mainline-descriptors 4) profile-id)
       => "real-policy-004/dynamic-rewrite")
(check (.get (vector-ref vertical-mainline-descriptors 5) profile-id)
       => "real-policy-005/memory-recall")
(check (.get (vector-ref vertical-mainline-descriptors 6) profile-id)
       => "policy-combination/memory-rewrite-checker")

(check (vector-length projection-modules) => 9)
(check (.get (vector-ref projection-modules 0) kind)
       => "marlin.config-interface.loop-policy.profile-projection-module.v1")
(check (.get (vector-ref projection-modules 0) module-id)
       => "poo-flow.loop-engine.real-repair-001")
(check (.get (vector-ref projection-modules 1) module-id)
       => "poo-flow.loop-engine.failure-retry")
(check (.get (vector-ref projection-modules 2) module-id)
       => "poo-flow.loop-engine.policy-combination-matrix")
(check (.get (vector-ref projection-modules 3) module-id)
       => "poo-flow.loop-engine.real-policy-001-sandbox-denylist")
(check (.get (vector-ref projection-modules 4) module-id)
       => "poo-flow.loop-engine.real-policy-001-tool-sandbox")
(check (.get (vector-ref projection-modules 5) module-id)
       => "poo-flow.loop-engine.real-policy-002-retry-budget")
(check (.get (vector-ref projection-modules 6) module-id)
       => "poo-flow.loop-engine.real-policy-003-maker-checker")
(check (.get (vector-ref projection-modules 7) module-id)
       => "poo-flow.loop-engine.real-policy-004-dynamic-rewrite")
(check (.get (vector-ref projection-modules 8) module-id)
       => "poo-flow.loop-engine.real-policy-005-memory-recall")
(check (.get (vector-ref projection-modules 0) source-module)
       => ":config-interface/modules/policy-pack")
(check (.get (vector-ref projection-modules 1) poo-flow-module)
       => "loop-engine")
(check (vector-ref (.get (vector-ref projection-modules 0)
                         poo-flow-capability-lanes)
                   0)
       => "fun-flow")
(check (vector-ref (.get (vector-ref projection-modules 3)
                         poo-flow-capability-lanes)
                   1)
       => "sandbox")
(check (vector-ref (.get (vector-ref projection-modules 4)
                         poo-flow-capability-lanes)
                   2)
       => "tool-handoff")
(check (vector-ref (.get (vector-ref projection-modules 5)
                         poo-flow-capability-lanes)
                   1)
       => "retry")
(check (vector-ref (.get (vector-ref projection-modules 6)
                         poo-flow-capability-lanes)
                   2)
       => "checker")
(check (vector-ref (.get (vector-ref projection-modules 7)
                         poo-flow-capability-lanes)
                   1)
       => "rewrite")
(check (vector-ref (.get (vector-ref projection-modules 8)
                         poo-flow-capability-lanes)
                   1)
       => "memory")
(check (.get (vector-ref projection-modules 3) vertical-case-id)
       => "real-policy-001/sandbox-denylist")
(check (vector-ref (.get (vector-ref projection-modules 3)
                         vertical-capability-tags)
                   0)
       => '+sandbox)
(check (.get (vector-ref projection-modules 1) vertical-mainline?)
       => #f)
(check (.get (vector-ref projection-modules 2) rust-type)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get (.get (vector-ref projection-modules 0) compiler-receipt) profile-id)
       => "real-repair-001/reactive-tool-loop")
(check (.get (.get (vector-ref projection-modules 3) compiler-receipt) profile-id)
       => "real-policy-001/sandbox-denylist")
(check (.get (.get (.get (vector-ref projection-modules 4) compiler-receipt) loop-program) program_id)
       => "real-tool-sandbox-loop")
(check (.get (.get (.get (vector-ref projection-modules 5) compiler-receipt) loop-program) program_id)
       => "real-policy-002-retry-budget")
(check (.get (.get (.get (vector-ref projection-modules 6) compiler-receipt) loop-program) program_id)
       => "real-policy-003-maker-checker")
(check (.get (.get (.get (vector-ref projection-modules 7) compiler-receipt) loop-program) program_id)
       => "real-policy-004-dynamic-rewrite")
(check (.get (.get (.get (vector-ref projection-modules 8) compiler-receipt) loop-program) program_id)
       => "real-policy-005-memory-recall")

(check (vector-length profile-compiler-receipts) => 9)
(check (.get (vector-ref profile-compiler-receipts 0) profile-id)
       => "real-repair-001/reactive-tool-loop")
(check (.get (vector-ref profile-compiler-receipts 1) profile-id)
       => "marlin-failure-retry-profile/typed-recovery")
(check (.get (vector-ref profile-compiler-receipts 2) profile-id)
       => "policy-combination/memory-rewrite-checker")
(check (.get (vector-ref profile-compiler-receipts 3) profile-id)
       => "real-policy-001/sandbox-denylist")
(check (.get (vector-ref profile-compiler-receipts 4) profile-id)
       => "real-policy-001/tool-sandbox")
(check (.get (vector-ref profile-compiler-receipts 5) profile-id)
       => "real-policy-002/retry-budget")
(check (.get (vector-ref profile-compiler-receipts 6) profile-id)
       => "real-policy-003/maker-checker")
(check (.get (vector-ref profile-compiler-receipts 7) profile-id)
       => "real-policy-004/dynamic-rewrite")
(check (.get (vector-ref profile-compiler-receipts 8) profile-id)
       => "real-policy-005/memory-recall")
(check (.get (vector-ref profile-compiler-receipts 0) compiler-owner)
       => "gerbil-poo-flow")
(check (.get (vector-ref profile-compiler-receipts 1) scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get (vector-ref profile-compiler-receipts 2) serialization-boundary)
       => "rust-owned-cli-trace-cross-process")

(check (.get real-policy-003-compiler-receipt profile-id)
       => "real-policy-003/maker-checker")
(check (.get real-policy-003-policy-pack policy_epoch) => 12)
(check (vector-ref (.get real-policy-003-hot-pack maker_profiles) 0)
       => 30)
(check (vector-ref (.get real-policy-003-hot-pack checker_profiles) 0)
       => 31)
(check (vector-length (.get real-policy-003-loop-program transitions))
       => 3)
(check (.get (vector-ref (.get real-policy-003-loop-program transitions) 0)
             action)
       => "invoke_model")
(check (.get (vector-ref (.get real-policy-003-loop-program transitions) 1)
             action)
       => "verify")
(check (.get (vector-ref (.get real-policy-003-loop-program transitions) 2)
             action)
       => "stop")

(check (.get real-policy-004-compiler-receipt profile-id)
       => "real-policy-004/dynamic-rewrite")
(check (.get real-policy-004-policy-pack policy_epoch) => 13)
(check (vector-ref (.get real-policy-004-hot-pack checker_profiles) 0)
       => 40)
(check (vector-length (.get real-policy-004-loop-program transitions))
       => 4)
(check (.get (vector-ref (.get real-policy-004-loop-program transitions) 0)
             action)
       => "rewrite_graph")
(check (.get (vector-ref (.get real-policy-004-loop-program transitions) 1)
             action)
       => "dispatch_tools")
(check (.get (vector-ref (.get real-policy-004-loop-program transitions) 2)
             action)
       => "verify")
(check (.get (vector-ref (.get real-policy-004-loop-program transitions) 3)
             action)
       => "stop")

(check (.get real-policy-005-compiler-receipt profile-id)
       => "real-policy-005/memory-recall")
(check (.get real-policy-005-policy-pack policy_epoch) => 14)
(check (vector-length (.get real-policy-005-hot-pack continuation_table))
       => 1)
(check (.get (vector-ref (.get real-policy-005-hot-pack continuation_table) 0)
             op)
       => "stop_completed")
(check (vector-length (.get real-policy-005-loop-program transitions))
       => 3)
(check (.get (vector-ref (.get real-policy-005-loop-program transitions) 0)
             action)
       => "read_memory")
(check (.get (vector-ref (.get real-policy-005-loop-program transitions) 1)
             action)
       => "dispatch_tools")
(check (.get (vector-ref (.get real-policy-005-loop-program transitions) 2)
             action)
       => "stop")

(check (.get failure-retry-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get failure-retry-compiler-receipt profile-id)
       => "marlin-failure-retry-profile/typed-recovery")
(check (.get failure-retry-compiler-receipt compiler-owner)
       => "gerbil-poo-flow")
(check (.get failure-retry-compiler-receipt scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get failure-retry-compiler-receipt serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get failure-retry-policy-pack policy_epoch) => 21)
(check (.get failure-retry-budget-caps max_attempts) => 3)
(check (.get (vector-ref failure-retry-continuation-table 0) op) => "retry")
(check (.get (vector-ref failure-retry-continuation-table 0) graph_template)
       => 1)
(check (.get (vector-ref failure-retry-continuation-table 0) max_attempts)
       => 3)
(check (.get failure-retry-loop-program program_id)
       => "failure-retry-typed-recovery")
(check (vector-length (.get failure-retry-loop-program transitions)) => 5)

(check (.get real-repair-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get real-repair-compiler-receipt profile-id)
       => "real-repair-001/reactive-tool-loop")
(check (.get real-repair-compiler-receipt compiler-owner)
       => "gerbil-poo-flow")
(check (.get real-repair-compiler-receipt scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get real-repair-compiler-receipt serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get real-repair-policy-pack policy_epoch) => 42)
(check (vector-ref real-repair-policy-mixins 0)
       => "reactive-tool-loop-base")
(check (vector-ref real-repair-policy-mixins 6)
       => "artifact-policy")
(check (vector-ref real-repair-policy-mixins 7)
       => "trace-policy")
(check (vector-length real-repair-merge-receipts) => 5)
(check (.get (vector-ref real-repair-merge-receipts 0) merge)
       => "union")
(check (.get (vector-ref real-repair-merge-receipts 1) merge)
       => "intersection")
(check (.get (vector-ref real-repair-merge-receipts 2) merge)
       => "min")
(check (.get (vector-ref real-repair-merge-receipts 3) merge)
       => "ordered_append")
(check (.get (vector-ref real-repair-merge-receipts 4) merge)
       => "conflict_error")
(check (.get (vector-ref real-repair-merge-receipts 4) status)
       => "conflict")
(check (.get real-repair-loop-program program_id)
       => "real-repair-001-scripted-loop")
(check (vector-length (.get real-repair-loop-program transitions)) => 6)

(check (.get policy-combination-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get policy-combination-compiler-receipt profile-id)
       => "policy-combination/memory-rewrite-checker")
(check (.get policy-combination-compiler-receipt compiler-owner)
       => "gerbil-poo-flow")
(check (.get policy-combination-compiler-receipt scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get policy-combination-compiler-receipt serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get policy-combination-policy-pack policy_epoch) => 15)
(check (vector-length (.get policy-combination-hot-pack graph_nodes)) => 3)
(check (vector-length (.get policy-combination-audit-pack linearization)) => 5)
(check (.get policy-combination-loop-program program_id)
       => "policy-combination-memory-rewrite-checker")
(check (vector-length (.get policy-combination-loop-program mechanism_policies))
       => 3)
(check (vector-length (.get policy-combination-loop-program transitions))
       => 6)
(check (.get (vector-ref (.get policy-combination-loop-program transitions) 0)
             action)
       => "read_memory")
(check (.get (vector-ref (.get policy-combination-loop-program transitions) 5)
             action)
       => "stop")

(display "config-interface-loop-policy-pack-ok")
(newline)
