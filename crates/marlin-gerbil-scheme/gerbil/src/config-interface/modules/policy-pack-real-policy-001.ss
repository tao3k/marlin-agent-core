;;; -*- Gerbil -*-
;;; Engineering note: This module owns the first real policy cases and their
;;; shared transition constructor. Keep the constructor here so later profiles
;;; reuse the same Rust LoopProgram field spelling instead of inventing variants.
package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-pack-presentation
        :config-interface/modules/policy-pack-slot-merge
        :config-interface/modules/policy-pack-support)

(export marlin-real-policy-transition
        marlinRealPolicy001SandboxDenylistSlotMergeAlgebraReceipts
        marlinRealPolicy001SandboxDenylistResolvedPolicyPack
        marlinRealPolicy001SandboxDenylistLoopProgram
        marlinRealPolicy001SandboxDenylistLoopProgramCompilerReceipt
        marlinRealToolSandboxSlotMergeAlgebraReceipts
        marlinRealToolSandboxResolvedPolicyPack
        marlinRealToolSandboxLoopProgram
        marlinRealToolSandboxLoopProgramCompilerReceipt)

;;; Boundary:
;;; - real-policy-001 sandbox denylist stays as Scheme-owned profile data.
;;; - Rust receives only the compiled LoopProgram and resolved policy objects.
;; : (-> MarlinInput MarlinResult)
(def (marlin-real-policy-001-sandbox-denylist-mechanism-policies)
  (vector "real-policy-001-sandbox-denylist"
          "agent-flow-tool-projection"))

;;; Boundary:
;;; - Denylist mixins document the policy roles participating in this profile.
;;; - Keep the role vector local so audit provenance matches the digest inputs.
;; : (-> StringVector)
(def (marlin-real-policy-001-sandbox-denylist-policy-mixins)
  (vector "sandbox-denylist-policy"
          "artifact-policy"
          "trace-policy"))

;;; Boundary:
;;; - The digest binds policy data to the Rust LoopProgram replay contract.
;;; - Changing any hot or audit field should update this digest input string.
;; : (-> ByteVector)
(def (marlin-real-policy-001-sandbox-denylist-digest)
  (marlin-policy-digest
   "real-policy-001/sandbox-denylist"
   10
   (marlin-real-policy-001-sandbox-denylist-mechanism-policies)
   (marlin-real-policy-001-sandbox-denylist-policy-mixins)
   "capability=3;human-gate=0;attempts=1;exclusive=true;continuation=stop_failed"
   "linearization=sandbox-denylist,runtime-kernel;merges=union"))

;;; Boundary:
;;; - Tool sandbox mechanism policy ids stay separate from the denylist case.
;;; - The profile catalog can compare both variants without inspecting objects.
;; : (-> StringVector)
(def (marlin-real-policy-001-tool-sandbox-mechanism-policies)
  (vector "real-policy-001-tool-sandbox"
          "agent-flow-tool-projection"))

;;; Boundary:
;;; - Tool sandbox mixins describe the allowed path rather than the deny path.
;;; - Keep the role names stable because Rust-side audit receipts quote them.
;; : (-> StringVector)
(def (marlin-real-policy-001-tool-sandbox-policy-mixins)
  (vector "tool-sandbox-policy"
          "artifact-policy"
          "trace-policy"))

;;; Boundary:
;;; - The tool sandbox digest is a distinct replay key from the denylist digest.
;;; - It preserves the allowed continuation path for downstream comparisons.
;; : (-> ByteVector)
(def (marlin-real-policy-001-tool-sandbox-digest)
  (marlin-policy-digest
   "real-policy-001/tool-sandbox"
   10
   (marlin-real-policy-001-tool-sandbox-mechanism-policies)
   (marlin-real-policy-001-tool-sandbox-policy-mixins)
   "capability=3;human-gate=0;attempts=1;exclusive=false;continuation=stop_completed"
   "linearization=tool-sandbox,runtime-kernel;merges=intersection"))

;;; Boundary:
;;; - Transition construction owns Rust LoopProgram field spelling.
;;; - Later policy modules import this helper to avoid drift across profiles.
;; : (-> TransitionId StateId EventId ActionId StateId LoopTransition)
(def (marlin-real-policy-transition transition-id-value
                                    from-value
                                    event-value
                                    action-value
                                    to-value)
  (.o transition_id: transition-id-value
      from: from-value
      event: event-value
      action: action-value
      to: to-value))

;;; Boundary:
;;; - Denylist slot merge receipts preserve the union merge semantics.
;;; - These receipts feed both forced slot evidence and audit merge receipts.
;; : (-> SlotMergeReceiptVector)
(def (marlinRealPolicy001SandboxDenylistSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeUnion
    10
    "sandbox.denylist"
    (vector "secrets/.env" ".env")
    (vector "target/" "secrets/.env"))))

;;; Boundary:
;;; - Tool sandbox slot receipts preserve the intersection merge semantics.
;;; - Keep this vector separate from denylist receipts so audits stay readable.
;; : (-> SlotMergeReceiptVector)
(def (marlinRealToolSandboxSlotMergeAlgebraReceipts)
  (vector
   (marlinPolicySlotMergeIntersection
    11
    "capability"
    (vector "+tool" "+sandbox" "+spawn")
    (vector "+tool" "+sandbox" "+runtime"))))

;;; Boundary:
;;; - Resolved denylist policy pack is the hot Rust-facing policy payload.
;;; - Audit fields remain adjacent so replay evidence follows the same profile.
;; : (-> ResolvedPolicyPack)
(def (marlinRealPolicy001SandboxDenylistResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 10
      policy_digest: (marlin-real-policy-001-sandbox-denylist-digest)
      hot:
      (.o capability_mask: 3
          human_gate_mask: 0
          budget_caps:
          (.o max_attempts: 1
              max_cost_units: 100
              max_wall_time_ms: 5000)
          graph_nodes:
          (vector
           (.o node_id: 10
               executor_id: 20
               capability_mask: 3
               resource_class_id: 30))
          graph_edges: (vector)
          route_index:
          (.o buckets:
              (vector
               (.o bucket_id: 10
                   scope_mask: 255
                   target_id: 20)))
          resource_classes:
          (vector
           (.o resource_class_id: 30
               exclusive: #t))
          continuation_table:
          (vector
           (.o op: "stop_failed"))
          maker_profiles: (vector)
          checker_profiles: (vector))
      audit:
      (.o policy_mixins:
          (marlin-real-policy-001-sandbox-denylist-policy-mixins)
          provenance:
          (vector
           (.o slot_id: 10
               winner_role: "sandbox-denylist"
               source_role_order: (vector "sandbox-denylist" "runtime-kernel")
               merge: "union"))
          linearization: (vector "sandbox-denylist" "runtime-kernel")
          diagnostics:
          (vector
           (.o code: "real-policy-001-sandbox-denylist-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 10
               path: "gerbil/src/config-interface/modules/policy-pack.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector "real-policy-001 projects Scheme-authored sandbox denylist into typed loop program")
          forced_slots:
          (marlinPolicySlotMergeForcedSlots
           (marlinRealPolicy001SandboxDenylistSlotMergeAlgebraReceipts)
           "hot")
          merge_receipts:
          (marlinPolicySlotMergeAuditReceipts
           (marlinRealPolicy001SandboxDenylistSlotMergeAlgebraReceipts)))))

;;; Boundary:
;;; - Denylist LoopProgram stops on the denied tool path.
;;; - Transition ids are stable because receipts may cite them by name.
;; : (-> LoopProgram)
(def (marlinRealPolicy001SandboxDenylistLoopProgram)
  (.o schema_version: 1
      program_id: "real-policy-001-sandbox-denylist"
      policy_epoch: 10
      policy_digest: (marlin-real-policy-001-sandbox-denylist-digest)
      mechanism_policies:
      (marlin-real-policy-001-sandbox-denylist-mechanism-policies)
      initial_state: "start"
      transitions:
      (vector
       (marlin-real-policy-transition
        "start-tool"
        "start"
        "start"
        "dispatch_tools"
        "await-tool")
       (marlin-real-policy-transition
        "tool-denied-stop"
        "await-tool"
        "error"
        "stop"
        "stopped"))))

;;; Boundary:
;;; - Compiler receipts bind the resolved pack and LoopProgram as one artifact.
;;; - The profile id must match the catalog descriptor exactly.
;; : (-> LoopProgramCompilerReceipt)
(def (marlinRealPolicy001SandboxDenylistLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-policy-001/sandbox-denylist"
   (marlinRealPolicy001SandboxDenylistResolvedPolicyPack)
   (marlinRealPolicy001SandboxDenylistLoopProgram)))

;;; Boundary:
;;; - Allowed tool+sandbox profile mirrors the denylist shape for comparison.
;;; - The hot payload differs only where the policy semantics require it.
;; : (-> ResolvedPolicyPack)
(def (marlinRealToolSandboxResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 10
      policy_digest: (marlin-real-policy-001-tool-sandbox-digest)
      hot:
      (.o capability_mask: 3
          human_gate_mask: 0
          budget_caps:
          (.o max_attempts: 1
              max_cost_units: 100
              max_wall_time_ms: 5000)
          graph_nodes:
          (vector
           (.o node_id: 11
               executor_id: 21
               capability_mask: 3
               resource_class_id: 31))
          graph_edges: (vector)
          route_index:
          (.o buckets:
              (vector
               (.o bucket_id: 11
                   scope_mask: 255
                   target_id: 21)))
          resource_classes:
          (vector
           (.o resource_class_id: 31
               exclusive: #f))
          continuation_table:
          (vector
           (.o op: "stop_completed"))
          maker_profiles: (vector)
          checker_profiles: (vector))
      audit:
      (.o policy_mixins:
          (marlin-real-policy-001-tool-sandbox-policy-mixins)
          provenance:
          (vector
           (.o slot_id: 11
               winner_role: "tool-sandbox"
               source_role_order: (vector "tool-sandbox" "runtime-kernel")
               merge: "intersection"))
          linearization: (vector "tool-sandbox" "runtime-kernel")
          diagnostics:
          (vector
           (.o code: "real-policy-001-tool-sandbox-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 11
               path: "gerbil/src/config-interface/modules/policy-pack.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector "real-policy-001 projects Scheme-authored tool sandbox spawn into typed loop program")
          forced_slots:
          (marlinPolicySlotMergeForcedSlots
           (marlinRealToolSandboxSlotMergeAlgebraReceipts)
           "hot")
          merge_receipts:
          (marlinPolicySlotMergeAuditReceipts
           (marlinRealToolSandboxSlotMergeAlgebraReceipts)))))

;;; Boundary:
;;; - Tool sandbox LoopProgram completes after the tool receipt.
;;; - It stays separate from denylist flow to keep negative and positive cases clear.
;; : (-> LoopProgram)
(def (marlinRealToolSandboxLoopProgram)
  (.o schema_version: 1
      program_id: "real-tool-sandbox-loop"
      policy_epoch: 10
      policy_digest: (marlin-real-policy-001-tool-sandbox-digest)
      mechanism_policies:
      (marlin-real-policy-001-tool-sandbox-mechanism-policies)
      initial_state: "start"
      transitions:
      (vector
       (marlin-real-policy-transition
        "start-tool"
        "start"
        "start"
        "dispatch_tools"
        "await-tool")
       (marlin-real-policy-transition
        "tool-stop"
        "await-tool"
        "tool_receipt"
        "stop"
        "stopped"))))

;;; Boundary:
;;; - Compiler receipt exports the allowed tool sandbox profile to the catalog.
;;; - The public function remains thin so profile data is the review surface.
;; : (-> LoopProgramCompilerReceipt)
(def (marlinRealToolSandboxLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-policy-001/tool-sandbox"
   (marlinRealToolSandboxResolvedPolicyPack)
   (marlinRealToolSandboxLoopProgram)))
