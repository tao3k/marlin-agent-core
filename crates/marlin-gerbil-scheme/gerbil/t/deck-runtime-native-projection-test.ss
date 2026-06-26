;;; -*- Gerbil -*-
;;; Boundary: Test owns the Gerbil POO typed projection contract used by Rust.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-native-projection
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def projection-test-policy
  (make-marlin-deck-runtime-model-route-policy
   "customer-extension"
   "openai"
   "gpt-5.4"
   '("codex")
   '("extension-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-native-projection-contract)
  (let (projection
        (marlin-deck-runtime-project-poo-policy
         projection-test-policy
         "register"))
    (check marlin-deck-runtime-native-projection-abi-id
           => "marlin.deck-runtime.native-projection")
    (check marlin-deck-runtime-native-projection-abi-version => 1)
    (check marlin-deck-runtime-poo-policy-projection-symbol
           => "marlin_deck_runtime_project_poo_policy")
    (check (.get projection type_id)
           => marlin-deck-runtime-poo-policy-projection-type-id)
    (check (.get projection schema_id)
           => marlin-deck-runtime-poo-policy-projection-schema-id)
    (check (.get projection policy_id) => "customer-extension")
    (check (.get projection object_system)
           => marlin-deck-runtime-poo-package-name)
    (check (.get projection package) => marlin-deck-runtime-package-name)
    (check (.get projection module)
           => ":marlin/deck-runtime-native-projection")
    (check (.get projection action) => "register")))

(def (check-resolved-loop-policy-pack-contract)
  (let* ((pack (make-marlin-deck-runtime-sample-resolved-loop-policy-pack))
         (hot (.get pack hot))
         (audit (.get pack audit))
         (graph-node (car (.get hot graph_nodes)))
         (provenance (car (.get audit provenance)))
         (merge-receipt (car (.get audit merge_receipts))))
    (check marlin-deck-runtime-resolved-loop-policy-pack-type-id
           => "marlin.loop-policy.resolved-pack")
    (check marlin-deck-runtime-resolved-loop-policy-pack-schema-id
           => "marlin.loop-policy.resolved-pack.v1")
    (check marlin-deck-runtime-resolved-loop-policy-pack-symbol
           => "marlin_deck_runtime_project_resolved_loop_policy_pack")
    (check (.get pack schema_version) => 1)
    (check (.get pack policy_epoch) => 42)
    (check (length (.get pack policy_digest)) => 32)
    (check (car (.get pack policy_digest)) => 7)
    (check (.get hot capability_mask) => 5)
    (check (.get hot human_gate_mask) => 1)
    (check (.get graph-node node_id) => 1)
    (check (.get graph-node executor_id) => 2)
    (check (.get provenance winner_role) => "planner")
    (check (.get provenance source_role_order) => '("planner" "reviewer"))
    (check (.get merge-receipt status) => "applied")
    (check (marlin-deck-runtime-project-resolved-loop-policy-pack pack)
           => pack)))

(check-native-projection-contract)
(check-resolved-loop-policy-pack-contract)
