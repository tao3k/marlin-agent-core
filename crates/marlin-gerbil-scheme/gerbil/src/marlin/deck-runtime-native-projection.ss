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
        make-marlin-deck-runtime-poo-policy-projection
        marlin-deck-runtime-project-poo-policy)

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
