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

(check-native-projection-contract)
