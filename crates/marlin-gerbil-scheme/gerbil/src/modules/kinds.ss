;;; -*- Gerbil -*-
;;; Boundary: Shared module-system kind ids and prototypes.

package: modules

(import (only-in :clan/poo/object .o))

(export marlin-modules-kind
        marlin-module-workflow-kind
        marlin-module-catalog-kind
        marlin-eval-modules-result-kind
        marlin-module-system-presentation-kind
        marlin-module-projection-chain-kind
        marlin-policy-extension-kind
        marlin-policy-module-kind
        marlin-policy-module-workflow-kind
        marlin-policy-substrate-gate-kind
        marlin-policy-pack-kind
        marlin-pack-catalog-kind
        marlin-policy-projection-kind
        marlin-policy-pack-presentation-kind
        marlin-policy-pack-inventory-kind
        marlin-policy-object-kind
        marlin-policy-object-operation-kind
        marlin-policy-object-surgery-receipt-kind
        marlin-module-import-kind
        marlin-import-source-ref-kind
        marlin-import-local-source-kind
        marlin-module-prototype)

;;; Boundary: Public user module kind is stable across module-lib helpers.
;; MarlinResult <- MarlinInput
(def marlin-modules-kind
  "marlin.modules.v1")

;;; Boundary: Workflow helpers keep runtime projections out of user config files.
;; MarlinResult <- MarlinInput
(def marlin-module-workflow-kind
  "marlin.modules.workflow.v1")

;;; Boundary: Catalogs are Scheme values, not path/evaluator conventions.
;; MarlinResult <- MarlinInput
(def marlin-module-catalog-kind
  "marlin.modules.catalog.v1")

;;; Boundary: evalModules returns a receipt bundle, not raw runtime config.
;; MarlinResult <- MarlinInput
(def marlin-eval-modules-result-kind
  "marlin.modules.eval-result.v1")

;;; Boundary: Presentations are scalar receipts for the whole user module surface.
;; MarlinResult <- MarlinInput
(def marlin-module-system-presentation-kind
  "marlin.modules.system-presentation.v1")

;;; Boundary: Projection chains name stable Rust-owned receipt handoff points.
;; MarlinResult <- MarlinInput
(def marlin-module-projection-chain-kind
  "marlin.modules.projection-chain.v1")

;;; Boundary: User .ss files export POO extension objects managed by modules.
;; MarlinResult <- MarlinInput
(def marlin-policy-extension-kind
  "marlin.modules.policy-extension-object.v1")

;;; Boundary: Policy modules are Scheme-owned POO modules, not Rust DSLs.
;; MarlinResult <- MarlinInput
(def marlin-policy-module-kind
  "marlin.modules.policy-module.v1")

;;; Boundary: Policy workflows add policy substrate metadata to module workflow.
;; MarlinResult <- MarlinInput
(def marlin-policy-module-workflow-kind
  "marlin.modules.policy-workflow.v1")

;;; Boundary: The substrate gate is a stable receipt for Rust validation.
;; MarlinResult <- MarlinInput
(def marlin-policy-substrate-gate-kind
  "marlin.modules.policy-substrate-gate.v1")

;;; Boundary: Policy packs are Scheme/POO prefab bundles, not Rust DSLs.
;; MarlinResult <- MarlinInput
(def marlin-policy-pack-kind
  "marlin.modules.policy-pack.v1")

;;; Boundary: Pack catalogs collect prefab bundles without parsing .ss text.
;; MarlinResult <- MarlinInput
(def marlin-pack-catalog-kind
  "marlin.modules.policy-pack-catalog.v1")

;;; Boundary: Policy projections are fixed envelopes over Scheme POO output.
;; MarlinResult <- MarlinInput
(def marlin-policy-projection-kind
  "marlin.modules.policy-projection.v1")

;;; Boundary: Pack presentations are scalar receipts for Rust/debug tooling.
;; MarlinResult <- MarlinInput
(def marlin-policy-pack-presentation-kind
  "marlin.modules.policy-pack-presentation.v1")

;;; Boundary: Pack inventories list prefab furniture as a typed projection.
;; MarlinResult <- MarlinInput
(def marlin-policy-pack-inventory-kind
  "marlin.modules.policy-pack-inventory.v1")

;;; Boundary: Policy objects are the POO "furniture" inside prefab packs.
;; MarlinResult <- MarlinInput
(def marlin-policy-object-kind
  "marlin.modules.policy-object.v1")

;;; Boundary: Object surgery keeps add/remove/disable/replace in Scheme.
;; MarlinResult <- MarlinInput
(def marlin-policy-object-operation-kind
  "marlin.modules.policy-object-operation.v1")

;;; Boundary: Surgery receipts prove Scheme changed objects, not Rust handlers.
;; MarlinResult <- MarlinInput
(def marlin-policy-object-surgery-receipt-kind
  "marlin.modules.policy-object-surgery-receipt.v1")

;;; Boundary: Public import specs name user files and exported profiles.
;; MarlinResult <- MarlinInput
(def marlin-module-import-kind
  "marlin.modules.import.v1")

;;; Boundary: Import source references keep source metadata as POO objects.
;; MarlinResult <- MarlinInput
(def marlin-import-source-ref-kind
  "marlin.modules.import.source-ref.v1")

;;; Boundary: Local sources name user workspace files without parsing them.
;; MarlinResult <- MarlinInput
(def marlin-import-local-source-kind
  "marlin.modules.import.local-source.v1")

;;; Boundary: Internal prototype anchors module defaults for imported interfaces.
;; MarlinResult <- MarlinInput
(def marlin-module-prototype
  (.o kind: marlin-modules-kind
      id: "anonymous-marlin-module"
      imports: '()
      extensions: '()
      scripts: '()
      options: (.o)
      schemas: (.o)
      metadata: '()))
