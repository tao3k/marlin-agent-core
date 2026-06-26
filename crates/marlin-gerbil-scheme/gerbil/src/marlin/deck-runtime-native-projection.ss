;;; -*- Gerbil -*-
;;; Boundary: Module owns Gerbil POO typed projections for the Rust native ABI.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime)

(export marlin-deck-runtime-native-projection-abi-id
        marlin-deck-runtime-native-projection-abi-version
        marlin-deck-runtime-poo-policy-projection-type-id
        marlin-deck-runtime-poo-policy-projection-schema-id
        marlin-deck-runtime-poo-policy-projection-symbol
        marlin-deck-runtime-resolved-loop-policy-pack-type-id
        marlin-deck-runtime-resolved-loop-policy-pack-schema-id
        marlin-deck-runtime-resolved-loop-policy-pack-symbol
        make-marlin-deck-runtime-poo-policy-projection
        marlin-deck-runtime-project-poo-policy
        make-marlin-deck-runtime-resolved-loop-policy-hot-pack
        make-marlin-deck-runtime-resolved-loop-policy-audit-pack
        make-marlin-deck-runtime-resolved-loop-policy-pack
        make-marlin-deck-runtime-sample-resolved-loop-policy-pack
        marlin-deck-runtime-project-resolved-loop-policy-pack)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-native-projection-abi-id
  "marlin.deck-runtime.native-projection")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-native-projection-abi-version 1)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-poo-policy-projection-type-id
  "marlin.deck-runtime.poo-policy-projection")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-poo-policy-projection-schema-id
  "marlin.deck-runtime.poo-policy-projection.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-poo-policy-projection-symbol
  "marlin_deck_runtime_project_poo_policy")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-resolved-loop-policy-pack-type-id
  "marlin.loop-policy.resolved-pack")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-resolved-loop-policy-pack-schema-id
  "marlin.loop-policy.resolved-pack.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-resolved-loop-policy-pack-symbol
  "marlin_deck_runtime_project_resolved_loop_policy_pack")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-poo-policy-projection policy-id action-value)
  (.o type_id: marlin-deck-runtime-poo-policy-projection-type-id
      schema_id: marlin-deck-runtime-poo-policy-projection-schema-id
      policy_id: policy-id
      object_system: marlin-deck-runtime-poo-package-name
      package: marlin-deck-runtime-package-name
      module: ":marlin/deck-runtime-native-projection"
      action: action-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-project-poo-policy policy action-value)
  (make-marlin-deck-runtime-poo-policy-projection
   (.get policy name)
   action-value))

;;; Boundary: Hot loop pack stays runtime-shaped and omits audit strings.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-resolved-loop-policy-hot-pack)
  (.o capability_mask: 5
      human_gate_mask: 1
      budget_caps:
      (.o max_attempts: 3
          max_cost_units: 1000
          max_wall_time_ms: 30000)
      graph_nodes:
      (list (.o node_id: 1
                executor_id: 2
                capability_mask: 5
                resource_class_id: 4))
      graph_edges:
      (list (.o from: 1
                to: 2))
      route_index:
      (.o buckets:
          (list (.o bucket_id: 1
                    scope_mask: 255
                    target_id: 3)))
      resource_classes:
      (list (.o resource_class_id: 4
                exclusive: #t))
      continuation_table:
      (list (.o op: "stop_completed"))
      maker_profiles: (list 11)
      checker_profiles: (list 12)))

;;; Boundary: Audit loop pack owns explanations and provenance outside the hot path.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-resolved-loop-policy-audit-pack)
  (.o provenance:
      (list (.o slot_id: 9
                winner_role: "planner"
                source_role_order: (list "planner" "reviewer")
                merge: "union"))
      linearization: (list "planner" "reviewer")
      diagnostics:
      (list (.o code: "policy-pack-ok"
                severity: "info"))
      source_locations:
      (list (.o source_location_id: 1
                path: "gerbil/src/config-interface/modules/policy-pack.ss"
                line: 10
                column: 2))
      explanation_strings:
      (list "forced policy pack before native handoff")
      forced_slots:
      (list (.o slot_id: 9
                hotness: "hot"))
      merge_receipts:
      (list (.o slot_id: 9
                merge: "union"
                status: "applied"))))

;;; Boundary: Payload fields mirror Rust ResolvedLoopPolicyPack exactly.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-resolved-loop-policy-pack
      schema-version-value
      policy-epoch-value
      policy-digest-value
      hot-value
      audit-value)
  (.o schema_version: schema-version-value
      policy_epoch: policy-epoch-value
      policy_digest: policy-digest-value
      hot: hot-value
      audit: audit-value))

;;; Boundary: Sample pack is a deterministic Scheme-side projection fixture.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-sample-resolved-loop-policy-pack)
  (make-marlin-deck-runtime-resolved-loop-policy-pack
   1
   42
   (make-list 32 7)
   (make-marlin-deck-runtime-resolved-loop-policy-hot-pack)
   (make-marlin-deck-runtime-resolved-loop-policy-audit-pack)))

;;; Boundary: Native projection symbol returns payload; Rust owns the typed envelope.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-project-resolved-loop-policy-pack pack)
  pack)
