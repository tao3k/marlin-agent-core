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

;; MarlinResult <- MarlinInput
(def (digest-varies? digest-value)
  (let ((first-byte (vector-ref digest-value 0))
        (count (vector-length digest-value)))
    (let loop ((index 1))
      (if (< index count)
        (if (= (vector-ref digest-value index) first-byte)
          (loop (+ index 1))
          #t)
        #f))))

(def failure-retry-compiler-receipt
  (marlinFailureRetryLoopProgramCompilerReceipt))

(def failure-retry-policy-pack
  (.get failure-retry-compiler-receipt resolved-policy-pack))

(def failure-retry-audit-pack
  (.get failure-retry-policy-pack audit))

(def failure-retry-merge-receipts
  (.get failure-retry-audit-pack merge_receipts))

(def failure-retry-slot-merge-algebra-receipts
  (marlinFailureRetrySlotMergeAlgebraReceipts))

(def failure-retry-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   failure-retry-slot-merge-algebra-receipts))

(def failure-retry-policy-digest
  (.get failure-retry-policy-pack policy_digest))

(def failure-retry-hot-pack
  (.get failure-retry-policy-pack hot))

(def failure-retry-budget-caps
  (.get failure-retry-hot-pack budget_caps))

(def failure-retry-continuation-table
  (.get failure-retry-hot-pack continuation_table))

(def failure-retry-loop-program
  (.get failure-retry-compiler-receipt loop-program))

(def failure-retry-loop-policy-digest
  (.get failure-retry-loop-program policy_digest))

(def real-repair-compiler-receipt
  (marlinRealRepair001LoopProgramCompilerReceipt))

(def real-repair-policy-pack
  (.get real-repair-compiler-receipt resolved-policy-pack))

(def real-repair-policy-digest
  (.get real-repair-policy-pack policy_digest))

(def real-repair-audit-pack
  (.get real-repair-policy-pack audit))

(def real-repair-policy-mixins
  (.get real-repair-audit-pack policy_mixins))

(def real-repair-merge-receipts
  (.get real-repair-audit-pack merge_receipts))

(def real-repair-slot-merge-algebra-receipts
  (marlinRealRepair001SlotMergeAlgebraReceipts))

(def real-repair-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-repair-slot-merge-algebra-receipts))

(def real-repair-loop-program
  (.get real-repair-compiler-receipt loop-program))

(def real-repair-loop-policy-digest
  (.get real-repair-loop-program policy_digest))

(def policy-combination-compiler-receipt
  (marlinPolicyCombinationMatrixLoopProgramCompilerReceipt))

(def policy-combination-policy-pack
  (.get policy-combination-compiler-receipt resolved-policy-pack))

(def policy-combination-policy-digest
  (.get policy-combination-policy-pack policy_digest))

(def policy-combination-hot-pack
  (.get policy-combination-policy-pack hot))

(def policy-combination-audit-pack
  (.get policy-combination-policy-pack audit))

(def policy-combination-merge-receipts
  (.get policy-combination-audit-pack merge_receipts))

(def policy-combination-slot-merge-algebra-receipts
  (marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts))

(def policy-combination-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   policy-combination-slot-merge-algebra-receipts))

(def policy-combination-loop-program
  (.get policy-combination-compiler-receipt loop-program))

(def policy-combination-loop-policy-digest
  (.get policy-combination-loop-program policy_digest))

(def slot-merge-algebra-demo
  (marlinPolicySlotMergeAlgebraDemoReceipt))

(def slot-merge-receipts
  (.get slot-merge-algebra-demo receipts))

(def slot-merge-capability
  (vector-ref slot-merge-receipts 0))

(def slot-merge-denylist
  (vector-ref slot-merge-receipts 1))

(def slot-merge-human-gates
  (vector-ref slot-merge-receipts 2))

(def slot-merge-budget
  (vector-ref slot-merge-receipts 3))

(def slot-merge-route-rules
  (vector-ref slot-merge-receipts 4))

(def slot-merge-exclusive-resource
  (vector-ref slot-merge-receipts 5))

(def slot-merge-observability
  (vector-ref slot-merge-receipts 6))

(def projection-modules
  (marlinLoopPolicyProjectionModules))

(def projection-descriptors
  (marlinLoopPolicyProfileProjectionDescriptors))

(def vertical-mainline-descriptors
  (marlinLoopVerticalMainlineProjectionDescriptors))

(def profile-compiler-receipts
  (marlinLoopPolicyProfileCompilerReceipts))

(def real-policy-001-sandbox-compiler-receipt
  (vector-ref profile-compiler-receipts 3))

(def real-policy-001-sandbox-policy-pack
  (.get real-policy-001-sandbox-compiler-receipt resolved-policy-pack))

(def real-policy-001-sandbox-loop-program
  (.get real-policy-001-sandbox-compiler-receipt loop-program))

(def real-policy-001-sandbox-policy-digest
  (.get real-policy-001-sandbox-policy-pack policy_digest))

(def real-policy-001-sandbox-audit-pack
  (.get real-policy-001-sandbox-policy-pack audit))

(def real-policy-001-sandbox-merge-receipts
  (.get real-policy-001-sandbox-audit-pack merge_receipts))

(def real-policy-001-sandbox-slot-merge-algebra-receipts
  (marlinRealPolicy001SandboxDenylistSlotMergeAlgebraReceipts))

(def real-policy-001-sandbox-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-policy-001-sandbox-slot-merge-algebra-receipts))

(def real-policy-001-tool-compiler-receipt
  (vector-ref profile-compiler-receipts 4))

(def real-policy-001-tool-policy-pack
  (.get real-policy-001-tool-compiler-receipt resolved-policy-pack))

(def real-policy-001-tool-loop-program
  (.get real-policy-001-tool-compiler-receipt loop-program))

(def real-policy-001-tool-policy-digest
  (.get real-policy-001-tool-policy-pack policy_digest))

(def real-policy-001-tool-audit-pack
  (.get real-policy-001-tool-policy-pack audit))

(def real-policy-001-tool-merge-receipts
  (.get real-policy-001-tool-audit-pack merge_receipts))

(def real-policy-001-tool-slot-merge-algebra-receipts
  (marlinRealToolSandboxSlotMergeAlgebraReceipts))

(def real-policy-001-tool-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-policy-001-tool-slot-merge-algebra-receipts))

(def real-policy-002-compiler-receipt
  (vector-ref profile-compiler-receipts 5))

(def real-policy-002-policy-pack
  (.get real-policy-002-compiler-receipt resolved-policy-pack))

(def real-policy-002-audit-pack
  (.get real-policy-002-policy-pack audit))

(def real-policy-002-merge-receipts
  (.get real-policy-002-audit-pack merge_receipts))

(def real-policy-002-slot-merge-algebra-receipts
  (marlinRealPolicy002RetryBudgetSlotMergeAlgebraReceipts))

(def real-policy-002-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-policy-002-slot-merge-algebra-receipts))

(def real-policy-002-hot-pack
  (.get real-policy-002-policy-pack hot))

(def real-policy-002-loop-program
  (.get real-policy-002-compiler-receipt loop-program))

(def real-policy-003-compiler-receipt
  (vector-ref profile-compiler-receipts 6))

(def real-policy-003-policy-pack
  (.get real-policy-003-compiler-receipt resolved-policy-pack))

(def real-policy-003-audit-pack
  (.get real-policy-003-policy-pack audit))

(def real-policy-003-merge-receipts
  (.get real-policy-003-audit-pack merge_receipts))

(def real-policy-003-slot-merge-algebra-receipts
  (marlinRealPolicy003MakerCheckerSlotMergeAlgebraReceipts))

(def real-policy-003-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-policy-003-slot-merge-algebra-receipts))

(def real-policy-003-hot-pack
  (.get real-policy-003-policy-pack hot))

(def real-policy-003-loop-program
  (.get real-policy-003-compiler-receipt loop-program))

(def real-policy-004-compiler-receipt
  (vector-ref profile-compiler-receipts 7))

(def real-policy-004-policy-pack
  (.get real-policy-004-compiler-receipt resolved-policy-pack))

(def real-policy-004-audit-pack
  (.get real-policy-004-policy-pack audit))

(def real-policy-004-merge-receipts
  (.get real-policy-004-audit-pack merge_receipts))

(def real-policy-004-slot-merge-algebra-receipts
  (marlinRealPolicy004DynamicRewriteSlotMergeAlgebraReceipts))

(def real-policy-004-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-policy-004-slot-merge-algebra-receipts))

(def real-policy-004-hot-pack
  (.get real-policy-004-policy-pack hot))

(def real-policy-004-loop-program
  (.get real-policy-004-compiler-receipt loop-program))

(def real-policy-005-compiler-receipt
  (vector-ref profile-compiler-receipts 8))

(def real-policy-005-policy-pack
  (.get real-policy-005-compiler-receipt resolved-policy-pack))

(def real-policy-005-audit-pack
  (.get real-policy-005-policy-pack audit))

(def real-policy-005-merge-receipts
  (.get real-policy-005-audit-pack merge_receipts))

(def real-policy-005-slot-merge-algebra-receipts
  (marlinRealPolicy005MemoryRecallSlotMergeAlgebraReceipts))

(def real-policy-005-slot-merge-audit-receipts
  (marlinPolicySlotMergeAuditReceipts
   real-policy-005-slot-merge-algebra-receipts))

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

(check (.get slot-merge-algebra-demo kind)
       => marlin-policy-slot-merge-algebra-demo-receipt-kind)
(check (.get slot-merge-algebra-demo profile-id)
       => "policy-merge-algebra-demo")
(check (.get slot-merge-algebra-demo owner)
       => "poo-flow.scheme")
(check (.get slot-merge-algebra-demo receipt-count) => 7)
(check (vector-length slot-merge-receipts) => 7)
(check (vector-length (.get slot-merge-algebra-demo required-laws)) => 7)
(check (.get slot-merge-algebra-demo scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get slot-merge-algebra-demo serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get slot-merge-algebra-demo rust-handler-manufactured)
       => #f)

(check (.get slot-merge-capability kind)
       => marlin-policy-slot-merge-receipt-kind)
(check (.get slot-merge-capability slot) => "capability")
(check (.get slot-merge-capability merge) => "intersection")
(check (.get slot-merge-capability status) => "merged")
(check (vector-length (.get slot-merge-capability result)) => 2)
(check (vector-ref (.get slot-merge-capability result) 0)
       => "+read")
(check (vector-ref (.get slot-merge-capability result) 1)
       => "+tool")

(check (.get slot-merge-denylist slot) => "denylist")
(check (.get slot-merge-denylist merge) => "union")
(check (vector-length (.get slot-merge-denylist result)) => 2)
(check (vector-ref (.get slot-merge-denylist result) 0)
       => "secrets/.env")
(check (vector-ref (.get slot-merge-denylist result) 1)
       => "target/")

(check (.get slot-merge-human-gates slot) => "human_gates")
(check (.get slot-merge-human-gates merge) => "union")
(check (vector-length (.get slot-merge-human-gates result)) => 2)
(check (vector-ref (.get slot-merge-human-gates result) 0)
       => "security-review")
(check (vector-ref (.get slot-merge-human-gates result) 1)
       => "cost-review")

(check (.get slot-merge-budget slot) => "budget.max_attempts")
(check (.get slot-merge-budget merge) => "min")
(check (.get slot-merge-budget result) => 2)

(check (.get slot-merge-route-rules slot) => "route_rules")
(check (.get slot-merge-route-rules merge) => "ordered_append")
(check (vector-length (.get slot-merge-route-rules result)) => 4)
(check (vector-ref (.get slot-merge-route-rules result) 0)
       => "model")
(check (vector-ref (.get slot-merge-route-rules result) 3)
       => "stop")

(check (.get slot-merge-exclusive-resource slot)
       => "exclusive_resource")
(check (.get slot-merge-exclusive-resource merge)
       => "conflict_error")
(check (.get slot-merge-exclusive-resource status)
       => "conflict")
(check (.get slot-merge-exclusive-resource result) => #f)
(check (vector-ref (.get slot-merge-exclusive-resource conflict-reasons) 0)
       => "exclusive-resource-conflict")

(check (.get slot-merge-observability slot) => "observability")
(check (.get slot-merge-observability merge) => "union")
(check (vector-length (.get slot-merge-observability result)) => 2)
(check (vector-ref (.get slot-merge-observability result) 0)
       => "runtime.tool")
(check (vector-ref (.get slot-merge-observability result) 1)
       => "harness.execution")
(check (.get slot-merge-observability rust-handler-manufactured)
       => #f)

(check (vector-length real-policy-001-sandbox-policy-digest) => 32)
(check (digest-varies? real-policy-001-sandbox-policy-digest) => #t)
(check (.get real-policy-001-sandbox-loop-program policy_digest)
       => real-policy-001-sandbox-policy-digest)
(check (vector-length real-policy-001-tool-policy-digest) => 32)
(check (digest-varies? real-policy-001-tool-policy-digest) => #t)
(check (.get real-policy-001-tool-loop-program policy_digest)
       => real-policy-001-tool-policy-digest)
(check (equal? real-policy-001-sandbox-policy-digest
               real-policy-001-tool-policy-digest)
       => #f)
(check (vector-length real-policy-001-sandbox-slot-merge-algebra-receipts)
       => 1)
(check (.get (vector-ref real-policy-001-sandbox-slot-merge-algebra-receipts 0)
             slot_id)
       => 10)
(check (.get (vector-ref real-policy-001-sandbox-slot-merge-algebra-receipts 0)
             merge)
       => "union")
(check (vector-length
        (.get (vector-ref real-policy-001-sandbox-slot-merge-algebra-receipts 0)
              result))
       => 3)
(check (vector-ref
        (.get (vector-ref real-policy-001-sandbox-slot-merge-algebra-receipts 0)
              result)
        2)
       => "target/")
(check (vector-length (.get real-policy-001-sandbox-audit-pack forced_slots))
       => 1)
(check (vector-length real-policy-001-sandbox-merge-receipts) => 1)
(check (.get (vector-ref real-policy-001-sandbox-merge-receipts 0) merge)
       => (.get (vector-ref real-policy-001-sandbox-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref real-policy-001-sandbox-merge-receipts 0) status)
       => (.get (vector-ref real-policy-001-sandbox-slot-merge-audit-receipts 0)
                status))
(check (vector-length real-policy-001-tool-slot-merge-algebra-receipts)
       => 1)
(check (.get (vector-ref real-policy-001-tool-slot-merge-algebra-receipts 0)
             slot_id)
       => 11)
(check (.get (vector-ref real-policy-001-tool-slot-merge-algebra-receipts 0)
             merge)
       => "intersection")
(check (vector-length
        (.get (vector-ref real-policy-001-tool-slot-merge-algebra-receipts 0)
              result))
       => 2)
(check (vector-ref
        (.get (vector-ref real-policy-001-tool-slot-merge-algebra-receipts 0)
              result)
        1)
       => "+sandbox")
(check (vector-length (.get real-policy-001-tool-audit-pack forced_slots))
       => 1)
(check (vector-length real-policy-001-tool-merge-receipts) => 1)
(check (.get (vector-ref real-policy-001-tool-merge-receipts 0) merge)
       => (.get (vector-ref real-policy-001-tool-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref real-policy-001-tool-merge-receipts 0) status)
       => (.get (vector-ref real-policy-001-tool-slot-merge-audit-receipts 0)
                status))

(check (.get real-policy-002-compiler-receipt profile-id)
       => "real-policy-002/retry-budget")
(check (.get real-policy-002-policy-pack policy_epoch) => 11)
(check (.get (.get real-policy-002-hot-pack budget_caps) max_attempts)
       => 2)
(check (vector-length real-policy-002-slot-merge-algebra-receipts) => 3)
(check (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 0) slot)
       => "capability")
(check (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 0)
             merge)
       => "intersection")
(check (vector-length
        (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 0)
              result))
       => 2)
(check (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 1) slot)
       => "budget.max_attempts")
(check (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 1)
             result)
       => 2)
(check (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 2) slot)
       => "route_rules")
(check (vector-ref
        (.get (vector-ref real-policy-002-slot-merge-algebra-receipts 2)
              result)
        2)
       => "stop_failed")
(check (vector-length (.get real-policy-002-audit-pack forced_slots)) => 3)
(check (vector-length real-policy-002-merge-receipts) => 3)
(check (.get (vector-ref real-policy-002-merge-receipts 0) merge)
       => (.get (vector-ref real-policy-002-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref real-policy-002-merge-receipts 2) merge)
       => (.get (vector-ref real-policy-002-slot-merge-audit-receipts 2)
                merge))
(check (.get real-policy-002-loop-program program_id)
       => "real-policy-002-retry-budget")

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
(check (vector-length real-policy-003-slot-merge-algebra-receipts) => 3)
(check (.get (vector-ref real-policy-003-slot-merge-algebra-receipts 0) slot)
       => "capability")
(check (.get (vector-ref real-policy-003-slot-merge-algebra-receipts 1) slot)
       => "budget.max_attempts")
(check (.get (vector-ref real-policy-003-slot-merge-algebra-receipts 1)
             result)
       => 1)
(check (.get (vector-ref real-policy-003-slot-merge-algebra-receipts 2) slot)
       => "route_rules")
(check (vector-length (.get real-policy-003-audit-pack forced_slots)) => 3)
(check (vector-length real-policy-003-merge-receipts) => 3)
(check (.get (vector-ref real-policy-003-merge-receipts 0) merge)
       => (.get (vector-ref real-policy-003-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref real-policy-003-merge-receipts 2) merge)
       => (.get (vector-ref real-policy-003-slot-merge-audit-receipts 2)
                merge))

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
(check (vector-length real-policy-004-slot-merge-algebra-receipts) => 3)
(check (.get (vector-ref real-policy-004-slot-merge-algebra-receipts 0) slot)
       => "capability")
(check (vector-ref
        (.get (vector-ref real-policy-004-slot-merge-algebra-receipts 0)
              result)
        0)
       => "+rewrite")
(check (.get (vector-ref real-policy-004-slot-merge-algebra-receipts 1)
             result)
       => 1)
(check (vector-ref
        (.get (vector-ref real-policy-004-slot-merge-algebra-receipts 2)
              result)
        3)
       => "stop")
(check (vector-length (.get real-policy-004-audit-pack forced_slots)) => 3)
(check (vector-length real-policy-004-merge-receipts) => 3)
(check (.get (vector-ref real-policy-004-merge-receipts 0) merge)
       => (.get (vector-ref real-policy-004-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref real-policy-004-merge-receipts 2) merge)
       => (.get (vector-ref real-policy-004-slot-merge-audit-receipts 2)
                merge))

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
(check (vector-length real-policy-005-slot-merge-algebra-receipts) => 3)
(check (.get (vector-ref real-policy-005-slot-merge-algebra-receipts 0) slot)
       => "capability")
(check (vector-ref
        (.get (vector-ref real-policy-005-slot-merge-algebra-receipts 0)
              result)
        0)
       => "+memory")
(check (.get (vector-ref real-policy-005-slot-merge-algebra-receipts 1)
             result)
       => 1)
(check (vector-ref
        (.get (vector-ref real-policy-005-slot-merge-algebra-receipts 2)
              result)
        2)
       => "stop")
(check (vector-length (.get real-policy-005-audit-pack forced_slots)) => 3)
(check (vector-length real-policy-005-merge-receipts) => 3)
(check (.get (vector-ref real-policy-005-merge-receipts 0) merge)
       => (.get (vector-ref real-policy-005-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref real-policy-005-merge-receipts 2) merge)
       => (.get (vector-ref real-policy-005-slot-merge-audit-receipts 2)
                merge))

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
(check (vector-length failure-retry-policy-digest) => 32)
(check (digest-varies? failure-retry-policy-digest) => #t)
(check failure-retry-loop-policy-digest => failure-retry-policy-digest)
(check (vector-ref (.get failure-retry-audit-pack policy_mixins) 0)
       => "failure-observer-policy")
(check (vector-ref (.get failure-retry-audit-pack policy_mixins) 4)
       => "trace-policy")
(check (.get failure-retry-budget-caps max_attempts) => 3)
(check (.get (vector-ref failure-retry-continuation-table 0) op) => "retry")
(check (.get (vector-ref failure-retry-continuation-table 0) graph_template)
       => 1)
(check (.get (vector-ref failure-retry-continuation-table 0) max_attempts)
       => 3)
(check (.get failure-retry-loop-program program_id)
       => "failure-retry-typed-recovery")
(check (vector-length (.get failure-retry-loop-program transitions)) => 5)
(check (vector-length failure-retry-slot-merge-algebra-receipts) => 2)
(check (.get (vector-ref failure-retry-slot-merge-algebra-receipts 0) slot)
       => "budget.max_attempts")
(check (.get (vector-ref failure-retry-slot-merge-algebra-receipts 0) merge)
       => "min")
(check (.get (vector-ref failure-retry-slot-merge-algebra-receipts 0) result)
       => 3)
(check (.get (vector-ref failure-retry-slot-merge-algebra-receipts 1) slot)
       => "observability")
(check (.get (vector-ref failure-retry-slot-merge-algebra-receipts 1) merge)
       => "union")
(check (vector-length (.get failure-retry-audit-pack forced_slots)) => 2)
(check (vector-length failure-retry-merge-receipts) => 2)
(check (.get (vector-ref failure-retry-merge-receipts 0) slot_id)
       => (.get (vector-ref failure-retry-slot-merge-audit-receipts 0)
                slot_id))
(check (.get (vector-ref failure-retry-merge-receipts 0) merge)
       => (.get (vector-ref failure-retry-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref failure-retry-merge-receipts 1) slot_id)
       => (.get (vector-ref failure-retry-slot-merge-audit-receipts 1)
                slot_id))
(check (.get (vector-ref failure-retry-merge-receipts 1) merge)
       => (.get (vector-ref failure-retry-slot-merge-audit-receipts 1)
                merge))

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
(check (vector-length real-repair-policy-digest) => 32)
(check (digest-varies? real-repair-policy-digest) => #t)
(check real-repair-loop-policy-digest => real-repair-policy-digest)
(check (vector-ref real-repair-policy-mixins 0)
       => "reactive-tool-loop-base")
(check (vector-ref real-repair-policy-mixins 6)
       => "artifact-policy")
(check (vector-ref real-repair-policy-mixins 7)
       => "trace-policy")
(check (vector-length real-repair-slot-merge-algebra-receipts) => 5)
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 0) kind)
       => marlin-policy-slot-merge-receipt-kind)
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 0) slot)
       => "human_gates")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 0) merge)
       => "union")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 0) status)
       => "merged")
(check (vector-length
        (.get (vector-ref real-repair-slot-merge-algebra-receipts 0) result))
       => 2)
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 1) slot)
       => "capability")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 1) merge)
       => "intersection")
(check (vector-ref
        (.get (vector-ref real-repair-slot-merge-algebra-receipts 1) result)
        0)
       => "+read")
(check (vector-ref
        (.get (vector-ref real-repair-slot-merge-algebra-receipts 1) result)
        1)
       => "+tool")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 2) slot)
       => "budget.max_attempts")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 2) result)
       => 3)
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 3) slot)
       => "route_rules")
(check (vector-ref
        (.get (vector-ref real-repair-slot-merge-algebra-receipts 3) result)
        4)
       => "stop")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 4) slot)
       => "exclusive_resource")
(check (.get (vector-ref real-repair-slot-merge-algebra-receipts 4) status)
       => "conflict")
(check (vector-ref
        (.get (vector-ref real-repair-slot-merge-algebra-receipts 4)
              conflict-reasons)
        0)
       => "exclusive-resource-conflict")
(check (vector-length real-repair-slot-merge-audit-receipts) => 5)
(check (.get (vector-ref real-repair-slot-merge-audit-receipts 0) status)
       => "applied")
(check (.get (vector-ref real-repair-slot-merge-audit-receipts 4) status)
       => "conflict")
(check (.get (vector-ref real-repair-merge-receipts 0) slot_id)
       => (.get (vector-ref real-repair-slot-merge-audit-receipts 0) slot_id))
(check (.get (vector-ref real-repair-merge-receipts 0) merge)
       => (.get (vector-ref real-repair-slot-merge-audit-receipts 0) merge))
(check (.get (vector-ref real-repair-merge-receipts 0) status)
       => (.get (vector-ref real-repair-slot-merge-audit-receipts 0) status))
(check (.get (vector-ref real-repair-merge-receipts 4) slot_id)
       => (.get (vector-ref real-repair-slot-merge-audit-receipts 4) slot_id))
(check (.get (vector-ref real-repair-merge-receipts 4) merge)
       => (.get (vector-ref real-repair-slot-merge-audit-receipts 4) merge))
(check (.get (vector-ref real-repair-merge-receipts 4) status)
       => (.get (vector-ref real-repair-slot-merge-audit-receipts 4) status))
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
(check (vector-length policy-combination-policy-digest) => 32)
(check (digest-varies? policy-combination-policy-digest) => #t)
(check policy-combination-loop-policy-digest
       => policy-combination-policy-digest)
(check (vector-length (.get policy-combination-hot-pack graph_nodes)) => 3)
(check (vector-ref (.get policy-combination-audit-pack policy_mixins) 0)
       => "memory-policy")
(check (vector-ref (.get policy-combination-audit-pack policy_mixins) 6)
       => "trace-policy")
(check (vector-length (.get policy-combination-audit-pack linearization)) => 5)
(check (vector-length policy-combination-slot-merge-algebra-receipts) => 5)
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 0) kind)
       => marlin-policy-slot-merge-receipt-kind)
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 0) slot)
       => "route_rules")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 0)
             merge)
       => "ordered_append")
(check (vector-length
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 0)
              result))
       => 6)
(check (vector-ref
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 0)
              result)
        0)
       => "read_memory")
(check (vector-ref
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 0)
              result)
        5)
       => "stop")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 1) slot)
       => "observability")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 1)
             merge)
       => "union")
(check (vector-length
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 1)
              result))
       => 4)
(check (vector-ref
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 1)
              result)
        2)
       => "runtime.tool")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 2) slot)
       => "budget.max_attempts")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 2)
             merge)
       => "min")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 2)
             result)
       => 3)
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 3) slot)
       => "capability")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 3)
             merge)
       => "intersection")
(check (vector-length
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 3)
              result))
       => 4)
(check (vector-ref
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 3)
              result)
        0)
       => "+memory")
(check (vector-ref
        (.get (vector-ref policy-combination-slot-merge-algebra-receipts 3)
              result)
        3)
       => "+verify")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 4) slot)
       => "human_gates")
(check (.get (vector-ref policy-combination-slot-merge-algebra-receipts 4)
             merge)
       => "union")
(check (vector-length policy-combination-slot-merge-audit-receipts) => 5)
(check (.get (vector-ref policy-combination-slot-merge-audit-receipts 0)
             status)
       => "applied")
(check (vector-length policy-combination-merge-receipts) => 5)
(check (.get (vector-ref policy-combination-merge-receipts 0) slot_id)
       => (.get (vector-ref policy-combination-slot-merge-audit-receipts 0)
                slot_id))
(check (.get (vector-ref policy-combination-merge-receipts 0) merge)
       => (.get (vector-ref policy-combination-slot-merge-audit-receipts 0)
                merge))
(check (.get (vector-ref policy-combination-merge-receipts 4) slot_id)
       => (.get (vector-ref policy-combination-slot-merge-audit-receipts 4)
                slot_id))
(check (.get (vector-ref policy-combination-merge-receipts 4) merge)
       => (.get (vector-ref policy-combination-slot-merge-audit-receipts 4)
                merge))
(check (vector-length (.get policy-combination-audit-pack forced_slots)) => 5)
(check (.get (vector-ref (.get policy-combination-audit-pack forced_slots) 4)
             hotness)
       => "audit_only")
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
