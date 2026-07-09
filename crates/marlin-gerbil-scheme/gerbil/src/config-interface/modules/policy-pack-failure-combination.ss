;;; -*- Gerbil -*-
;;; Engineering note: Failure and combination profiles stress multi-policy merge
;;; behavior. Transition tables stay declarative so the hard part is the policy
;;; interaction, not repeated object-constructor scaffolding.
package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-pack-presentation
        :config-interface/modules/policy-pack-slot-merge
        :config-interface/modules/policy-pack-support)

(export marlinFailureRetrySlotMergeAlgebraReceipts
        marlinFailureRetryResolvedPolicyPack
        marlinFailureRetryLoopProgram
        marlinFailureRetryLoopProgramCompilerReceipt
        marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts
        marlinPolicyCombinationMatrixResolvedPolicyPack
        marlinPolicyCombinationMatrixLoopProgram
        marlinPolicyCombinationMatrixLoopProgramCompilerReceipt)

;;; Boundary: Failure-retry profile compiled as Scheme types for Rust.
;; marlin-failure-retry-mechanism-policies
;;   : (-> StringVector)
(def (marlin-failure-retry-mechanism-policies)
  (vector "failure-retry-budget"
          "typed-recovery"
          "verification-gate"))

;; marlin-failure-retry-policy-mixins
;;   : (-> StringVector)
(def (marlin-failure-retry-policy-mixins)
  (vector "failure-observer-policy"
          "retry-governor-policy"
          "typed-recovery-policy"
          "artifact-policy"
          "trace-policy"))

;; marlinFailureRetrySlotMergeAlgebraReceipts
;;   : (-> SlotMergeReceiptVector)
(def (marlinFailureRetrySlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeMin
    21
    "budget.max_attempts"
    5
    3)
   (marlinPolicySlotMergeUnion
    22
    "observability"
    (vector "runtime.failure" "runtime.retry")
    (vector "harness.execution" "runtime.retry"))))

;; marlin-failure-retry-policy-digest
;;   : (-> ByteVector)
(def (marlin-failure-retry-policy-digest)
  (marlin-policy-digest
   "marlin-failure-retry-profile/typed-recovery"
   21
   (marlin-failure-retry-mechanism-policies)
   (marlin-failure-retry-policy-mixins)
   "capability=7;human-gate=0;attempts=3;cost=300;wall=15000;nodes=2;edges=1;maker=21;checker=22"
   "linearization=failure-observer,retry-governor,runtime-kernel;merges=min,union"))

;;; Boundary: Failure-retry transitions use Rust-owned LoopProgram IR field names.
;; marlin-failure-retry-transition
;;   : (-> TransitionId StateId EventId ActionId StateId LoopTransition)
(def (marlin-failure-retry-transition transition-id-value
                                      from-value
                                      event-value
                                      action-value
                                      to-value)
  (.o transition_id: transition-id-value
      from: from-value
      event: event-value
      action: action-value
      to: to-value))

;;; Boundary: Transition tables keep profile programs declarative and compact.
;; marlin-policy-transition-vector
;;   : (-> Procedure TransitionSpecList LoopTransitionVector)
(def (marlin-policy-transition-vector transition-constructor transition-specs)
  (list->vector
   (map (lambda (transition-spec)
          (apply transition-constructor transition-spec))
        transition-specs)))

;;; Boundary: Failure-retry resolved pack carries retry budget in hot IR.
;; marlinFailureRetryResolvedPolicyPack
;;   : (-> ResolvedPolicyPack)
(def (marlinFailureRetryResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 21
      policy_digest: (marlin-failure-retry-policy-digest)
      hot:
      (.o capability_mask: 7
          human_gate_mask: 0
          budget_caps:
          (.o max_attempts: 3
              max_cost_units: 300
              max_wall_time_ms: 15000)
          graph_nodes:
          (vector
           (.o node_id: 21
               executor_id: 31
               capability_mask: 7
               resource_class_id: 41)
           (.o node_id: 22
               executor_id: 32
               capability_mask: 3
               resource_class_id: 41))
          graph_edges:
          (vector
           (.o from: 21
               to: 22))
          route_index:
          (.o buckets:
              (vector
               (.o bucket_id: 21
                   scope_mask: 127
                   target_id: 31)))
          resource_classes:
          (vector
           (.o resource_class_id: 41
               exclusive: #t))
          continuation_table:
          (vector
           (.o op: "retry"
               graph_template: 1
               max_attempts: 3)
           (.o op: "stop_failed"))
          maker_profiles: (vector 21)
          checker_profiles: (vector 22))
      audit:
      (.o policy_mixins:
          (marlin-failure-retry-policy-mixins)
          provenance:
          (vector
           (.o slot_id: 21
               winner_role: "retry-governor"
               source_role_order: (vector "failure-observer" "retry-governor")
               merge: "min")
           (.o slot_id: 22
               winner_role: "runtime-kernel"
               source_role_order: (vector "retry-governor" "runtime-kernel")
               merge: "union"))
          linearization:
          (vector "failure-observer" "retry-governor" "runtime-kernel")
          diagnostics:
          (vector
           (.o code: "failure-retry-policy-pack-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 21
               path: "gerbil/src/config-interface/custom/marline-kernel/policies/loops/profiles/failure-retry.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector
           "failure-retry projects POO retry budget into typed loop program")
          forced_slots:
          (marlinPolicySlotMergeForcedSlots
           (marlinFailureRetrySlotMergeAlgebraReceipts)
           "hot")
          merge_receipts:
          (marlinPolicySlotMergeAuditReceipts
           (marlinFailureRetrySlotMergeAlgebraReceipts)))))

;;; Boundary: Failure-retry LoopProgram emits retry continuation handoff.
;; marlinFailureRetryLoopProgram
;;   : (-> LoopProgram)
(def (marlinFailureRetryLoopProgram)
  (.o schema_version: 1
      program_id: "failure-retry-typed-recovery"
      policy_epoch: 21
      policy_digest: (marlin-failure-retry-policy-digest)
      mechanism_policies:
      (marlin-failure-retry-mechanism-policies)
      initial_state: "start"
      transitions:
      (marlin-policy-transition-vector
       marlin-failure-retry-transition
       '(("start-classify-failure"
          "start"
          "start"
          "invoke_model"
          "await-classification")
         ("classification-plan-retry"
          "await-classification"
          "model_event"
          "runtime_handoff"
          "retry-planned")
         ("retry-plan-dispatch"
          "retry-planned"
          "runtime_receipt"
          "dispatch_tools"
          "await-retry-tool")
         ("retry-tool-verify"
          "await-retry-tool"
          "tool_receipt"
          "verify"
          "await-verification")
         ("verification-stop"
          "await-verification"
          "verification_receipt"
          "stop"
          "stopped")))))

;;; Boundary: Public compiler entrypoint for typed failure retry loops.
;; marlinFailureRetryLoopProgramCompilerReceipt
;;   : (-> LoopProgramCompilerReceipt)
(def (marlinFailureRetryLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "marlin-failure-retry-profile/typed-recovery"
   (marlinFailureRetryResolvedPolicyPack)
   (marlinFailureRetryLoopProgram)))

;;; Boundary: Policy combination profile exercises memory, maker, rewrite, tool, checker.
;; marlin-policy-combination-matrix-mechanism-policies
;;   : (-> StringVector)
(def (marlin-policy-combination-matrix-mechanism-policies)
  (vector "real-policy-003-maker-checker"
          "real-policy-004-dynamic-rewrite"
          "real-policy-005-memory-recall"))

;; marlin-policy-combination-matrix-policy-mixins
;;   : (-> StringVector)
(def (marlin-policy-combination-matrix-policy-mixins)
  (vector "memory-policy"
          "maker-policy"
          "dynamic-rewrite-policy"
          "tool-policy"
          "checker-policy"
          "artifact-policy"
          "trace-policy"))

;; marlin-policy-combination-matrix-policy-digest
;;   : (-> ByteVector)
(def (marlin-policy-combination-matrix-policy-digest)
  (marlin-policy-digest
   "policy-combination/memory-rewrite-checker"
   15
   (marlin-policy-combination-matrix-mechanism-policies)
   (marlin-policy-combination-matrix-policy-mixins)
   "capability=7;human-gate=1;attempts=3;cost=1000;wall=30000;nodes=3;edges=2;maker=21;checker=22;exclusive-resource=workspace-write"
   "linearization=memory,maker,rewrite,tool,checker;merges=ordered_append,union,min,intersection,union,conflict_error"))

;; marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts
;;   : (-> SlotMergeReceiptVector)
(def (marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeOrderedAppend
    31
    "route_rules"
    (vector "read_memory" "invoke_model")
    (vector "rewrite_graph" "dispatch_tools" "verify" "stop"))
   (marlinPolicySlotMergeUnion
    32
    "observability"
    (vector "runtime.memory" "runtime.model")
    (vector "runtime.tool" "harness.execution" "runtime.model"))
   (marlinPolicySlotMergeMin
    33
    "budget.max_attempts"
    5
    3)
   (marlinPolicySlotMergeIntersection
    34
    "capability"
    (vector "+memory" "+model" "+rewrite" "+tool" "+verify")
    (vector "+memory" "+rewrite" "+tool" "+verify" "+audit"))
   (marlinPolicySlotMergeUnion
    35
    "human_gates"
    (vector "checker-review")
    (vector "rewrite-review"))
   (marlinPolicySlotMergeConflictError
    36
    "exclusive_resource"
    "workspace-write"
    "workspace-write")))

;;; Boundary: Combination transitions stay in Rust-owned LoopProgram IR names.
;; marlin-policy-combination-matrix-transition
;;   : (-> TransitionId StateId EventId ActionId StateId LoopTransition)
(def (marlin-policy-combination-matrix-transition transition-id-value
                                                   from-value
                                                   event-value
                                                   action-value
                                                   to-value)
  (.o transition_id: transition-id-value
      from: from-value
      event: event-value
      action: action-value
      to: to-value))

;;; Boundary: Real policy combination pack preserves hot and audit evidence.
;; marlinPolicyCombinationMatrixResolvedPolicyPack
;;   : (-> ResolvedPolicyPack)
(def (marlinPolicyCombinationMatrixResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 15
      policy_digest: (marlin-policy-combination-matrix-policy-digest)
      hot:
      (.o capability_mask: 7
          human_gate_mask: 1
          budget_caps:
          (.o max_attempts: 3
              max_cost_units: 1000
              max_wall_time_ms: 30000)
          graph_nodes:
          (vector
           (.o node_id: 1
               executor_id: 21
               capability_mask: 1
               resource_class_id: 4)
           (.o node_id: 2
               executor_id: 22
               capability_mask: 2
               resource_class_id: 4)
           (.o node_id: 3
               executor_id: 23
               capability_mask: 4
               resource_class_id: 4))
          graph_edges:
          (vector
           (.o from: 1
               to: 2)
           (.o from: 2
               to: 3))
          route_index:
          (.o buckets:
              (vector
               (.o bucket_id: 1
                   scope_mask: 255
                   target_id: 3)))
          resource_classes:
          (vector
           (.o resource_class_id: 4
               exclusive: #t))
          continuation_table:
          (vector
           (.o op: "memory_rewrite_checker_stop"))
          maker_profiles: (vector 21)
          checker_profiles: (vector 22))
      audit:
      (.o policy_mixins:
          (marlin-policy-combination-matrix-policy-mixins)
          provenance:
          (vector
           (.o slot_id: 31
               winner_role: "memory"
               source_role_order: (vector "memory" "maker" "checker")
               merge: "ordered_append")
           (.o slot_id: 32
               winner_role: "maker"
               source_role_order: (vector "memory" "maker" "checker")
               merge: "union")
           (.o slot_id: 33
               winner_role: "checker"
               source_role_order: (vector "memory" "maker" "checker")
               merge: "min")
           (.o slot_id: 34
               winner_role: "checker"
               source_role_order: (vector "maker" "rewrite" "tool" "checker")
               merge: "intersection")
           (.o slot_id: 35
               winner_role: "checker"
               source_role_order: (vector "rewrite" "checker")
               merge: "union")
           (.o slot_id: 36
               winner_role: "tool"
               source_role_order: (vector "tool" "checker")
               merge: "conflict_error"))
          linearization: (vector "memory" "maker" "rewrite" "tool" "checker")
          diagnostics:
          (vector
           (.o code: "policy-combination-matrix-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 2
               path: "gerbil/src/config-interface/modules/policy-pack.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector "policy combination matrix projects memory, maker, rewrite, tool, checker into typed loop program")
          forced_slots:
          (vector
           (.o slot_id: 31
               hotness: "hot")
           (.o slot_id: 32
               hotness: "hot")
           (.o slot_id: 33
               hotness: "hot")
           (.o slot_id: 34
               hotness: "hot")
           (.o slot_id: 35
               hotness: "audit_only")
           (.o slot_id: 36
               hotness: "hot"))
          merge_receipts:
          (marlinPolicySlotMergeAuditReceipts
           (marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts)))))

;;; Boundary: Policy combination LoopProgram is emitted by the Scheme compiler surface.
;; marlinPolicyCombinationMatrixLoopProgram
;;   : (-> LoopProgram)
(def (marlinPolicyCombinationMatrixLoopProgram)
  (.o schema_version: 1
      program_id: "policy-combination-memory-rewrite-checker"
      policy_epoch: 15
      policy_digest: (marlin-policy-combination-matrix-policy-digest)
      mechanism_policies:
      (marlin-policy-combination-matrix-mechanism-policies)
      initial_state: "start"
      transitions:
      (marlin-policy-transition-vector
       marlin-policy-combination-matrix-transition
       '(("start-memory"
          "start"
          "start"
          "read_memory"
          "memory-ready")
         ("memory-maker"
          "memory-ready"
          "runtime_receipt"
          "invoke_model"
          "await-maker")
         ("maker-rewrite"
          "await-maker"
          "model_event"
          "rewrite_graph"
          "rewritten")
         ("rewrite-tool"
          "rewritten"
          "runtime_receipt"
          "dispatch_tools"
          "await-tool")
         ("tool-checker"
          "await-tool"
          "tool_receipt"
          "verify"
          "await-checker")
         ("checker-stop"
          "await-checker"
          "verification_receipt"
          "stop"
          "stopped")))))

;;; Boundary: Public compiler entrypoint for the first policy combination case.
;; marlinPolicyCombinationMatrixLoopProgramCompilerReceipt
;;   : (-> LoopProgramCompilerReceipt)
(def (marlinPolicyCombinationMatrixLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "policy-combination/memory-rewrite-checker"
   (marlinPolicyCombinationMatrixResolvedPolicyPack)
   (marlinPolicyCombinationMatrixLoopProgram)))
