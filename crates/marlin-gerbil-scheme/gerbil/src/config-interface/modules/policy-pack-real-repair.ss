;;; -*- Gerbil -*-
;;; Engineering note: Real repair fixtures are executable policy examples, not
;;; loose snapshots. Keep transitions and receipts close to the profile because
;;; they document the Rust LoopProgram contract that downstream tests exercise.
package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-pack-presentation
        :config-interface/modules/policy-pack-slot-merge
        :config-interface/modules/policy-pack-support)

(export marlinRealRepair001SlotMergeAlgebraReceipts
        marlinRealRepair001ResolvedPolicyPack
        marlinRealRepair001LoopProgram
        marlinRealRepair001LoopProgramCompilerReceipt)

;;; Boundary: Minimal real repair profile compiled as Scheme types for Rust.
;; : (-> MarlinInput MarlinResult)
(def (marlin-real-repair-001-mechanism-policies)
  (vector "reactive-tool-loop-base"
          "dynamic-graph-rewrite"
          "verification-gate"))

;; MarlinResult <- MarlinInput
(def (marlin-real-repair-001-policy-mixins)
  (vector "reactive-tool-loop-base"
          "workspace-write-policy"
          "sandbox-denylist-policy"
          "retry-budget-policy"
          "maker-policy"
          "checker-policy"
          "artifact-policy"
          "trace-policy"))

;; MarlinResult <- MarlinInput
(def (marlinRealRepair001SlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeUnion
    9
    "human_gates"
    (vector "planner-review")
    (vector "checker-review"))
   (marlinPolicySlotMergeIntersection
    10
    "capability"
    (vector "+read" "+write" "+tool")
    (vector "+read" "+tool" "+verify"))
   (marlinPolicySlotMergeMin
    11
    "budget.max_attempts"
    5
    3)
   (marlinPolicySlotMergeOrderedAppend
    12
    "route_rules"
    (vector "invoke_model" "dispatch_tools")
    (vector "rewrite_graph" "verify" "stop"))
   (marlinPolicySlotMergeConflictError
    13
    "exclusive_resource"
    "workspace-write"
    "repo-admin")))

;; MarlinResult <- MarlinInput
(def (marlin-real-repair-001-policy-digest)
  (marlin-policy-digest
   "real-repair-001/reactive-tool-loop"
   42
   (marlin-real-repair-001-mechanism-policies)
   (marlin-real-repair-001-policy-mixins)
   "capability=5;human-gate=1;attempts=3;cost=1000;wall=30000;nodes=1;edges=1;maker=11;checker=12"
   "linearization=planner,reviewer;merges=union,intersection,min,ordered_append,conflict_error"))

;;; Boundary: Loop transition values use Rust-owned IR field names.
;; MarlinResult <- MarlinInput
(def (marlin-real-repair-001-transition transition-id-value
                                       from-value
                                       event-value
                                       action-value
                                       to-value)
  (.o transition_id: transition-id-value
      from: from-value
      event: event-value
      action: action-value
      to: to-value))

;;; Boundary: First real policy pack projection for the vertical loop mainline.
;; MarlinResult <- MarlinInput
(def (marlinRealRepair001ResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 42
      policy_digest: (marlin-real-repair-001-policy-digest)
      hot:
      (.o capability_mask: 5
          human_gate_mask: 1
          budget_caps:
          (.o max_attempts: 3
              max_cost_units: 1000
              max_wall_time_ms: 30000)
          graph_nodes:
          (vector
           (.o node_id: 1
               executor_id: 2
               capability_mask: 5
               resource_class_id: 4))
          graph_edges:
          (vector
           (.o from: 1
               to: 2))
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
           (.o op: "stop_completed"))
          maker_profiles: (vector 11)
          checker_profiles: (vector 12))
      audit:
      (.o policy_mixins:
          (marlin-real-repair-001-policy-mixins)
          provenance:
          (vector
           (.o slot_id: 9
               winner_role: "planner"
               source_role_order: (vector "planner" "reviewer")
               merge: "union")
           (.o slot_id: 10
               winner_role: "reviewer"
               source_role_order: (vector "planner" "reviewer")
               merge: "intersection")
           (.o slot_id: 11
               winner_role: "budget"
               source_role_order: (vector "planner" "runtime-kernel")
               merge: "min")
           (.o slot_id: 12
               winner_role: "planner"
               source_role_order: (vector "planner" "runtime-kernel")
               merge: "ordered_append")
           (.o slot_id: 13
               winner_role: "runtime-kernel"
               source_role_order: (vector "workspace" "repo-admin")
               merge: "conflict_error"))
          linearization: (vector "planner" "reviewer")
          diagnostics:
          (vector
           (.o code: "real-repair-001-policy-pack-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 1
               path: "gerbil/src/config-interface/modules/policy-pack.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector "real-repair-001 projects POO profile into typed loop program")
          forced_slots:
          (marlinPolicySlotMergeForcedSlots
           (marlinRealRepair001SlotMergeAlgebraReceipts)
           "hot")
          merge_receipts:
          (marlinPolicySlotMergeAuditReceipts
           (marlinRealRepair001SlotMergeAlgebraReceipts)))))

;;; Boundary: First real LoopProgram emitted by the Scheme compiler surface.
;; MarlinResult <- MarlinInput
(def (marlinRealRepair001LoopProgram)
  (.o schema_version: 1
      program_id: "real-repair-001-scripted-loop"
      policy_epoch: 42
      policy_digest: (marlin-real-repair-001-policy-digest)
      mechanism_policies:
      (marlin-real-repair-001-mechanism-policies)
      initial_state: "start"
      transitions:
      (vector
       (marlin-real-repair-001-transition
        "start-model"
        "start"
        "start"
        "invoke_model"
        "await-model")
       (marlin-real-repair-001-transition
        "model-tools"
        "await-model"
        "tool_request"
        "dispatch_tools"
        "await-tools")
       (marlin-real-repair-001-transition
        "tools-continue"
        "await-tools"
        "tool_receipt"
        "continue"
        "await-model")
       (marlin-real-repair-001-transition
        "dynamic-rewrite"
        "await-model"
        "model_event"
        "rewrite_graph"
        "rewritten")
       (marlin-real-repair-001-transition
        "verify-rewrite"
        "rewritten"
        "runtime_receipt"
        "verify"
        "verifying")
       (marlin-real-repair-001-transition
        "verification-stop"
        "verifying"
        "verification_receipt"
        "stop"
        "stopped"))))

;;; Boundary: Public compiler entrypoint for the first vertical loop case.
;; MarlinResult <- MarlinInput
(def (marlinRealRepair001LoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-repair-001/reactive-tool-loop"
   (marlinRealRepair001ResolvedPolicyPack)
   (marlinRealRepair001LoopProgram)))
