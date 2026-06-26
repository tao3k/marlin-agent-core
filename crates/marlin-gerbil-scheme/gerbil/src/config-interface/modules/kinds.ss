;;; -*- Gerbil -*-
;;; Boundary: Marlin policy ids plus poo-flow-owned module-system ids.

package: config-interface/modules

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

(export marlin-config-interface-kind
        marlin-module-workflow-kind
        marlin-module-catalog-kind
        marlin-eval-modules-result-kind
        marlin-policy-facade-presentation-kind
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
        marlin-poo-loop-program-compiler-receipt-kind
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
(def marlin-config-interface-kind
  'poo-flow-module)

(def marlin-module-workflow-kind
  poo-flow-module-workflow-kind)

(def marlin-module-catalog-kind
  poo-flow-module-value-catalog-kind)

(def marlin-eval-modules-result-kind
  poo-flow-eval-modules-result-kind)

(def marlin-policy-facade-presentation-kind
  poo-flow-module-system-presentation-kind)

(def marlin-module-projection-chain-kind
  "marlin.config-interface.projection-chain.v1")

(def marlin-policy-extension-kind
  "marlin.config-interface.policy-extension-object.v1")

(def marlin-policy-module-kind
  "marlin.config-interface.policy-module.v1")

(def marlin-policy-module-workflow-kind
  "marlin.config-interface.policy-workflow.v1")

(def marlin-policy-substrate-gate-kind
  "marlin.config-interface.policy-substrate-gate.v1")

(def marlin-policy-pack-kind
  "marlin.config-interface.policy-pack.v1")

(def marlin-pack-catalog-kind
  "marlin.config-interface.policy-pack-catalog.v1")

(def marlin-pack-catalog-presentation-kind
  "marlin.config-interface.policy-pack-catalog-presentation.v1")

(def marlin-policy-projection-kind
  "marlin.config-interface.policy-projection.v1")

(def marlin-policy-projection-chain-receipt-kind
  "marlin.config-interface.policy-projection-chain-receipt.v1")

(def marlin-policy-budget-receipt-kind
  "marlin.runtime.policy-budget-receipt.v1")

(def marlin-policy-catalog-resolution-receipt-kind
  "marlin.runtime.policy-catalog-resolution-receipt.v1")

(def marlin-poo-loop-program-compiler-receipt-kind
  "marlin.config-interface.poo.loop-program-compiler-receipt.v1")

(def marlin-policy-pack-presentation-kind
  "marlin.config-interface.policy-pack-presentation.v1")

(def marlin-policy-pack-inventory-kind
  "marlin.config-interface.policy-pack-inventory.v1")

(def marlin-policy-object-kind
  "marlin.config-interface.policy-object.v1")

(def marlin-policy-object-operation-kind
  "marlin.config-interface.policy-object-operation.v1")

(def marlin-policy-object-surgery-receipt-kind
  "marlin.config-interface.policy-object-surgery-receipt.v1")

(def marlin-module-import-kind
  poo-flow-module-import-kind)

(def marlin-import-source-ref-kind
  poo-flow-module-import-source-ref-kind)

(def marlin-import-local-source-kind
  poo-flow-module-import-local-source-kind)

(def marlin-module-prototype
  poo-flow-module-descriptor-prototype)
