;;; -*- Gerbil -*-
;;; Boundary: Marlin policy ids plus poo-flow-owned module-system ids.

package: marlin/modules

(import (only-in :poo-flow/src/module-system/facade
                 poo-flow-modules-kind
                 poo-flow-module-workflow-kind
                 poo-flow-module-value-catalog-kind
                 poo-flow-eval-modules-result-kind
                 poo-flow-module-system-presentation-kind
                 poo-flow-module-import-kind
                 poo-flow-module-import-source-ref-kind
                 poo-flow-module-import-local-source-kind
                 poo-flow-module-descriptor-prototype))

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
        marlin-pack-catalog-presentation-kind
        marlin-policy-projection-chain-receipt-kind
        marlin-policy-budget-receipt-kind
        marlin-policy-catalog-resolution-receipt-kind
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

;;; Boundary: Descriptor `kind` follows poo-flow's role tag; `module-kind`
;;; carries the stable poo-flow receipt id.
(def marlin-modules-kind
  'poo-flow-module)

(def marlin-module-workflow-kind
  poo-flow-module-workflow-kind)

(def marlin-module-catalog-kind
  poo-flow-module-value-catalog-kind)

(def marlin-eval-modules-result-kind
  poo-flow-eval-modules-result-kind)

(def marlin-module-system-presentation-kind
  poo-flow-module-system-presentation-kind)

(def marlin-module-projection-chain-kind
  "marlin.modules.projection-chain.v1")

(def marlin-policy-extension-kind
  "marlin.modules.policy-extension-object.v1")

(def marlin-policy-module-kind
  "marlin.modules.policy-module.v1")

(def marlin-policy-module-workflow-kind
  "marlin.modules.policy-workflow.v1")

(def marlin-policy-substrate-gate-kind
  "marlin.modules.policy-substrate-gate.v1")

(def marlin-policy-pack-kind
  "marlin.modules.policy-pack.v1")

(def marlin-pack-catalog-kind
  "marlin.modules.policy-pack-catalog.v1")

(def marlin-pack-catalog-presentation-kind
  "marlin.modules.policy-pack-catalog-presentation.v1")

(def marlin-policy-projection-kind
  "marlin.modules.policy-projection.v1")

(def marlin-policy-projection-chain-receipt-kind
  "marlin.modules.policy-projection-chain-receipt.v1")

(def marlin-policy-budget-receipt-kind
  "marlin.runtime.policy-budget-receipt.v1")

(def marlin-policy-catalog-resolution-receipt-kind
  "marlin.runtime.policy-catalog-resolution-receipt.v1")

(def marlin-policy-pack-presentation-kind
  "marlin.modules.policy-pack-presentation.v1")

(def marlin-policy-pack-inventory-kind
  "marlin.modules.policy-pack-inventory.v1")

(def marlin-policy-object-kind
  "marlin.modules.policy-object.v1")

(def marlin-policy-object-operation-kind
  "marlin.modules.policy-object-operation.v1")

(def marlin-policy-object-surgery-receipt-kind
  "marlin.modules.policy-object-surgery-receipt.v1")

(def marlin-module-import-kind
  poo-flow-module-import-kind)

(def marlin-import-source-ref-kind
  poo-flow-module-import-source-ref-kind)

(def marlin-import-local-source-kind
  poo-flow-module-import-local-source-kind)

(def marlin-module-prototype
  poo-flow-module-descriptor-prototype)
