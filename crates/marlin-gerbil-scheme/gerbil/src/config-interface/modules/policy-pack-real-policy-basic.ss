;;; -*- Gerbil -*-
;;; Engineering note: Basic real-policy profiles are intentionally table-driven:
;;; the policy difference should live in data specs, while constructors preserve
;;; one stable Scheme-to-Rust object shape for all small profile variants.
package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-pack-presentation
        :config-interface/modules/policy-pack-slot-merge
        :config-interface/modules/policy-pack-support
        :config-interface/modules/policy-pack-real-policy-001)

(export marlinRealPolicy002RetryBudgetSlotMergeAlgebraReceipts
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
        marlinRealPolicy005MemoryRecallLoopProgramCompilerReceipt)

;;; Boundary: real-policy-002 retry-budget profile compiled by Scheme.
;; marlin-real-policy-basic-policy-mixins
;;   : (-> String StringVector)
(def (marlin-real-policy-basic-policy-mixins winner-role-value)
  (list->vector
   (map (lambda (policy-role)
          (if (string=? policy-role winner-role-value)
            (string-append policy-role "-policy")
            policy-role))
        (list winner-role-value "artifact-policy" "trace-policy"))))

;; marlin-real-policy-002-mechanism-policies
;;   : (-> StringVector)
(def (marlin-real-policy-002-mechanism-policies)
  (vector "real-policy-002-retry-budget"
          "agent-flow-tool-projection"))

;; marlin-real-policy-002-digest
;;   : (-> ByteVector)
(def (marlin-real-policy-002-digest)
  (marlin-policy-digest
   "real-policy-002/retry-budget"
   11
   (marlin-real-policy-002-mechanism-policies)
   (marlin-real-policy-basic-policy-mixins "retry-budget")
   "capability=3;human-gate=0;attempts=2;exclusive=true;continuation=retry,stop_failed"
   "linearization=retry-budget,runtime-kernel;merges=intersection,min,ordered_append"))

;; marlin-real-policy-003-mechanism-policies
;;   : (-> StringVector)
(def (marlin-real-policy-003-mechanism-policies)
  (vector "real-policy-003-maker-checker"))

;; marlin-real-policy-003-digest
;;   : (-> ByteVector)
(def (marlin-real-policy-003-digest)
  (marlin-policy-digest
   "real-policy-003/maker-checker"
   12
   (marlin-real-policy-003-mechanism-policies)
   (marlin-real-policy-basic-policy-mixins "maker-checker")
   "capability=5;human-gate=0;attempts=1;exclusive=false;continuation=stop_completed;maker=30;checker=31"
   "linearization=maker-checker,runtime-kernel;merges=intersection,min,ordered_append"))

;; marlin-real-policy-004-mechanism-policies
;;   : (-> StringVector)
(def (marlin-real-policy-004-mechanism-policies)
  (vector "real-policy-004-dynamic-rewrite"
          "verification-gate"))

;; marlin-real-policy-004-digest
;;   : (-> ByteVector)
(def (marlin-real-policy-004-digest)
  (marlin-policy-digest
   "real-policy-004/dynamic-rewrite"
   13
   (marlin-real-policy-004-mechanism-policies)
   (marlin-real-policy-basic-policy-mixins "dynamic-rewrite")
   "capability=7;human-gate=0;attempts=1;exclusive=true;continuation=stop_completed;checker=40"
   "linearization=dynamic-rewrite,runtime-kernel;merges=intersection,min,ordered_append"))

;; marlin-real-policy-005-mechanism-policies
;;   : (-> StringVector)
(def (marlin-real-policy-005-mechanism-policies)
  (vector "real-policy-005-memory-recall"
          "agent-flow-memory-projection"))

;; marlin-real-policy-005-digest
;;   : (-> ByteVector)
(def (marlin-real-policy-005-digest)
  (marlin-policy-digest
   "real-policy-005/memory-recall"
   14
   (marlin-real-policy-005-mechanism-policies)
   (marlin-real-policy-basic-policy-mixins "memory-recall")
   "capability=3;human-gate=0;attempts=1;exclusive=false;continuation=stop_completed"
   "linearization=memory-recall,runtime-kernel;merges=intersection,min,ordered_append"))

;; marlinRealPolicy002RetryBudgetSlotMergeAlgebraReceipts
;;   : (-> SlotMergeReceiptVector)
(def (marlinRealPolicy002RetryBudgetSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeIntersection
    110
    "capability"
    (vector "+tool" "+retry" "+verify")
    (vector "+tool" "+retry" "+dispatch"))
   (marlinPolicySlotMergeMin
    111
    "budget.max_attempts"
    4
    2)
   (marlinPolicySlotMergeOrderedAppend
    112
    "route_rules"
    (vector "dispatch_tools")
    (vector "retry" "stop_failed"))))

;; marlinRealPolicy003MakerCheckerSlotMergeAlgebraReceipts
;;   : (-> SlotMergeReceiptVector)
(def (marlinRealPolicy003MakerCheckerSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeIntersection
    120
    "capability"
    (vector "+model" "+verify" "+write")
    (vector "+model" "+verify" "+audit"))
   (marlinPolicySlotMergeMin
    121
    "budget.max_attempts"
    2
    1)
   (marlinPolicySlotMergeOrderedAppend
    122
    "route_rules"
    (vector "invoke_model")
    (vector "verify" "stop"))))

;; marlinRealPolicy004DynamicRewriteSlotMergeAlgebraReceipts
;;   : (-> SlotMergeReceiptVector)
(def (marlinRealPolicy004DynamicRewriteSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeIntersection
    130
    "capability"
    (vector "+rewrite" "+tool" "+verify" "+write")
    (vector "+rewrite" "+tool" "+verify"))
   (marlinPolicySlotMergeMin
    131
    "budget.max_attempts"
    2
    1)
   (marlinPolicySlotMergeOrderedAppend
    132
    "route_rules"
    (vector "rewrite_graph")
    (vector "dispatch_tools" "verify" "stop"))))

;; marlinRealPolicy005MemoryRecallSlotMergeAlgebraReceipts
;;   : (-> SlotMergeReceiptVector)
(def (marlinRealPolicy005MemoryRecallSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeIntersection
    140
    "capability"
    (vector "+memory" "+tool" "+verify")
    (vector "+memory" "+tool"))
   (marlinPolicySlotMergeMin
    141
    "budget.max_attempts"
    2
    1)
   (marlinPolicySlotMergeOrderedAppend
    142
    "route_rules"
    (vector "read_memory")
    (vector "dispatch_tools" "stop"))))

;; marlin-real-policy-basic-slot-merge-algebra-receipts
;;   : (-> String SlotMergeReceiptVector)
(def (marlin-real-policy-basic-slot-merge-algebra-receipts winner-role-value)
  (let (matches
        (filter (lambda (entry)
                  (string=? (car entry) winner-role-value))
                (list
                 (cons "retry-budget"
                       marlinRealPolicy002RetryBudgetSlotMergeAlgebraReceipts)
                 (cons "maker-checker"
                       marlinRealPolicy003MakerCheckerSlotMergeAlgebraReceipts)
                 (cons "dynamic-rewrite"
                       marlinRealPolicy004DynamicRewriteSlotMergeAlgebraReceipts)
                 (cons "memory-recall"
                       marlinRealPolicy005MemoryRecallSlotMergeAlgebraReceipts))))
    (if (pair? matches)
      ((cdr (car matches)))
      (error "unknown real policy slot merge algebra profile"
             winner-role-value))))

;;; Boundary: Continuation operations are alist specs projected into POO records.
;; marlin-real-policy-continuation-table
;;   : (-> ContinuationOperationSpecList ContinuationOperationVector)
(def (marlin-real-policy-continuation-table operation-specs)
  (list->vector
   (map marlin-policy-object<-alist operation-specs)))

;; marlin-real-policy-basic-resolved-policy-pack
;;   : (-> Integer ByteVector String Integer Integer Boolean Vector Vector
;;         Vector String String ResolvedPolicyPack)
(def (marlin-real-policy-basic-resolved-policy-pack policy-epoch-value
                                                     policy-digest-value
                                                     winner-role-value
                                                     capability-mask-value
                                                     max-attempts-value
                                                     exclusive-value
                                                     continuation-table-value
                                                     maker-profiles-value
                                                     checker-profiles-value
                                                     diagnostic-code-value
                                                     explanation-value)
  (let (slot-merge-algebra-receipts
        (marlin-real-policy-basic-slot-merge-algebra-receipts
         winner-role-value))
    (.o schema_version: 1
        policy_epoch: policy-epoch-value
        policy_digest: policy-digest-value
        hot:
        (.o capability_mask: capability-mask-value
            human_gate_mask: 0
            budget_caps:
            (.o max_attempts: max-attempts-value
                max_cost_units: 100
                max_wall_time_ms: 5000)
            graph_nodes:
            (vector
             (.o node_id: policy-epoch-value
                 executor_id: (+ policy-epoch-value 10)
                 capability_mask: capability-mask-value
                 resource_class_id: (+ policy-epoch-value 20)))
            graph_edges: (vector)
            route_index:
            (.o buckets:
                (vector
                 (.o bucket_id: policy-epoch-value
                     scope_mask: 255
                     target_id: (+ policy-epoch-value 10))))
            resource_classes:
            (vector
             (.o resource_class_id: (+ policy-epoch-value 20)
                 exclusive: exclusive-value))
            continuation_table: continuation-table-value
            maker_profiles: maker-profiles-value
            checker_profiles: checker-profiles-value)
        audit:
        (.o policy_mixins:
            (marlin-real-policy-basic-policy-mixins winner-role-value)
            provenance:
            (vector
             (.o slot_id: policy-epoch-value
                 winner_role: winner-role-value
                 source_role_order: (vector winner-role-value "runtime-kernel")
                 merge: "ordered_append"))
            linearization: (vector winner-role-value "runtime-kernel")
            diagnostics:
            (vector
             (.o code: diagnostic-code-value
                 severity: "info"))
            source_locations:
            (vector
             (.o source_location_id: policy-epoch-value
                 path: "gerbil/src/config-interface/modules/policy-pack.ss"
                 line: 1
                 column: 1))
            explanation_strings:
            (vector explanation-value)
            forced_slots:
            (marlinPolicySlotMergeForcedSlots slot-merge-algebra-receipts "hot")
            merge_receipts:
            (marlinPolicySlotMergeAuditReceipts
             slot-merge-algebra-receipts)))))

;; marlin-real-policy-loop-program
;;   : (-> ProgramId Integer ByteVector StringVector LoopTransitionVector
;;         LoopProgram)
(def (marlin-real-policy-loop-program program-id-value
                                      policy-epoch-value
                                      policy-digest-value
                                      mechanism-policies-value
                                      transitions-value)
  (.o schema_version: 1
      program_id: program-id-value
      policy_epoch: policy-epoch-value
      policy_digest: policy-digest-value
      mechanism_policies: mechanism-policies-value
      initial_state: "start"
      transitions: transitions-value))

;;; Boundary: Real policy transition tables are declarative specs plus one constructor.
;; marlin-real-policy-transition-vector
;;   : (-> TransitionSpecList LoopTransitionVector)
(def (marlin-real-policy-transition-vector transition-specs)
  (list->vector
   (map (lambda (transition-spec)
          (apply marlin-real-policy-transition transition-spec))
        transition-specs)))

;; marlinRealPolicy002RetryBudgetResolvedPolicyPack
;;   : (-> ResolvedPolicyPack)
(def (marlinRealPolicy002RetryBudgetResolvedPolicyPack)
  (marlin-real-policy-basic-resolved-policy-pack
   11
   (marlin-real-policy-002-digest)
   "retry-budget"
   3
   2
   #t
   (marlin-real-policy-continuation-table
    '(((op . "retry")
       (graph_template . 1)
       (max_attempts . 2))
      ((op . "stop_failed"))))
   (vector)
   (vector)
   "real-policy-002-retry-budget-ok"
   "real-policy-002 projects Scheme-authored retry budget into typed loop program"))

;; marlinRealPolicy002RetryBudgetLoopProgram
;;   : (-> LoopProgram)
(def (marlinRealPolicy002RetryBudgetLoopProgram)
  (marlin-real-policy-loop-program
   "real-policy-002-retry-budget"
   11
   (marlin-real-policy-002-digest)
   (marlin-real-policy-002-mechanism-policies)
   (marlin-real-policy-transition-vector
    '(("start-tool"
       "start"
       "start"
       "dispatch_tools"
       "await-tool")
      ("tool-error-retry"
       "await-tool"
       "error"
       "dispatch_tools"
       "await-tool-retry")
      ("retry-tool-stop"
       "await-tool-retry"
       "tool_receipt"
       "stop"
       "stopped")))))

;; marlinRealPolicy002RetryBudgetLoopProgramCompilerReceipt
;;   : (-> LoopProgramCompilerReceipt)
(def (marlinRealPolicy002RetryBudgetLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-policy-002/retry-budget"
   (marlinRealPolicy002RetryBudgetResolvedPolicyPack)
   (marlinRealPolicy002RetryBudgetLoopProgram)))

;;; Boundary: real-policy-003 maker/checker profile compiled by Scheme.
;; marlinRealPolicy003MakerCheckerResolvedPolicyPack
;;   : (-> ResolvedPolicyPack)
(def (marlinRealPolicy003MakerCheckerResolvedPolicyPack)
  (marlin-real-policy-basic-resolved-policy-pack
   12
   (marlin-real-policy-003-digest)
   "maker-checker"
   5
   1
   #f
   (marlin-real-policy-continuation-table
    '(((op . "stop_completed"))))
   (vector 30)
   (vector 31)
   "real-policy-003-maker-checker-ok"
   "real-policy-003 separates maker model and checker verification lanes"))

;; marlinRealPolicy003MakerCheckerLoopProgram
;;   : (-> LoopProgram)
(def (marlinRealPolicy003MakerCheckerLoopProgram)
  (marlin-real-policy-loop-program
   "real-policy-003-maker-checker"
   12
   (marlin-real-policy-003-digest)
   (marlin-real-policy-003-mechanism-policies)
   (marlin-real-policy-transition-vector
    '(("start-maker"
       "start"
       "start"
       "invoke_model"
       "await-maker")
      ("maker-checker"
       "await-maker"
       "model_event"
       "verify"
       "await-checker")
      ("checker-stop"
       "await-checker"
       "verification_receipt"
       "stop"
       "stopped")))))

;; marlinRealPolicy003MakerCheckerLoopProgramCompilerReceipt
;;   : (-> LoopProgramCompilerReceipt)
(def (marlinRealPolicy003MakerCheckerLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-policy-003/maker-checker"
   (marlinRealPolicy003MakerCheckerResolvedPolicyPack)
   (marlinRealPolicy003MakerCheckerLoopProgram)))

;;; Boundary: real-policy-004 dynamic rewrite profile compiled by Scheme.
;; marlinRealPolicy004DynamicRewriteResolvedPolicyPack
;;   : (-> ResolvedPolicyPack)
(def (marlinRealPolicy004DynamicRewriteResolvedPolicyPack)
  (marlin-real-policy-basic-resolved-policy-pack
   13
   (marlin-real-policy-004-digest)
   "dynamic-rewrite"
   7
   1
   #t
   (marlin-real-policy-continuation-table
    '(((op . "stop_completed"))))
   (vector)
   (vector 40)
   "real-policy-004-dynamic-rewrite-ok"
   "real-policy-004 rewrites the graph before repair and verification"))

;; marlinRealPolicy004DynamicRewriteLoopProgram
;;   : (-> LoopProgram)
(def (marlinRealPolicy004DynamicRewriteLoopProgram)
  (marlin-real-policy-loop-program
   "real-policy-004-dynamic-rewrite"
   13
   (marlin-real-policy-004-digest)
   (marlin-real-policy-004-mechanism-policies)
   (marlin-real-policy-transition-vector
    '(("start-rewrite"
       "start"
       "start"
       "rewrite_graph"
       "rewritten")
      ("rewrite-tool"
       "rewritten"
       "runtime_receipt"
       "dispatch_tools"
       "await-tool")
      ("tool-verify"
       "await-tool"
       "tool_receipt"
       "verify"
       "await-verification")
      ("verify-stop"
       "await-verification"
       "verification_receipt"
       "stop"
       "stopped")))))

;; marlinRealPolicy004DynamicRewriteLoopProgramCompilerReceipt
;;   : (-> LoopProgramCompilerReceipt)
(def (marlinRealPolicy004DynamicRewriteLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-policy-004/dynamic-rewrite"
   (marlinRealPolicy004DynamicRewriteResolvedPolicyPack)
   (marlinRealPolicy004DynamicRewriteLoopProgram)))

;;; Boundary: real-policy-005 memory recall profile compiled by Scheme.
;; marlinRealPolicy005MemoryRecallResolvedPolicyPack
;;   : (-> ResolvedPolicyPack)
(def (marlinRealPolicy005MemoryRecallResolvedPolicyPack)
  (marlin-real-policy-basic-resolved-policy-pack
   14
   (marlin-real-policy-005-digest)
   "memory-recall"
   3
   1
   #f
   (marlin-real-policy-continuation-table
    '(((op . "stop_completed"))))
   (vector)
   (vector)
   "real-policy-005-memory-recall-ok"
   "real-policy-005 uses memory recall to select the next tool path"))

;; marlinRealPolicy005MemoryRecallLoopProgram
;;   : (-> LoopProgram)
(def (marlinRealPolicy005MemoryRecallLoopProgram)
  (marlin-real-policy-loop-program
   "real-policy-005-memory-recall"
   14
   (marlin-real-policy-005-digest)
   (marlin-real-policy-005-mechanism-policies)
   (marlin-real-policy-transition-vector
    '(("start-memory"
       "start"
       "start"
       "read_memory"
       "memory-ready")
      ("memory-tool"
       "memory-ready"
       "tool_request"
       "dispatch_tools"
       "await-tool")
      ("tool-stop"
       "await-tool"
       "tool_receipt"
       "stop"
       "stopped")))))

;; marlinRealPolicy005MemoryRecallLoopProgramCompilerReceipt
;;   : (-> LoopProgramCompilerReceipt)
(def (marlinRealPolicy005MemoryRecallLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-policy-005/memory-recall"
   (marlinRealPolicy005MemoryRecallResolvedPolicyPack)
   (marlinRealPolicy005MemoryRecallLoopProgram)))
