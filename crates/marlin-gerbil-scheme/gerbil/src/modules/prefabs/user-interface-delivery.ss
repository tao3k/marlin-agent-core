;;; -*- Gerbil -*-
;;; Boundary: User-interface delivery receipt and Rust projection handoff.

package: modules/prefabs

(import (only-in :clan/poo/object .get .o)
        :modules/lib
        :modules/prefabs/user-interface)

(export UserInterface
        UserInterfaceWorkspace
        UserInterfaceDeliveryReceipt
        UserInterfaceApply
        UserInterfacePolicyProjection
        UserInterfaceOptionContracts
        user-interface-delivery-receipt-kind)

;;; Boundary: Delivery receipt kind is stable for user/debug surfaces.
;; MarlinResult <- MarlinInput
(def user-interface-delivery-receipt-kind
  "marlin.modules.prefabs.user-interface.delivery-receipt.v1")

;;; Boundary: Option contracts are shallow receipt facts for users/debug tools.
;; MarlinResult <- MarlinInput
(def (user-interface-option-contracts-from-workflow workflow)
  (map (lambda (receipt)
         (let ((option-id-value (.get receipt option-id))
               (source-module-id-value (.get receipt source-module-id))
               (valid-value (.get receipt valid?))
               (errors-value (.get receipt errors))
               (contract-kind-value (.get receipt contract-kind))
               (value-type-label-value (.get receipt value-type-label))
               (required-value (.get receipt required?))
               (optional-value (.get receipt optional?))
               (has-default-value (.get receipt has-default?))
               (default-value (.get receipt default))
               (has-constant-value (.get receipt has-constant?))
               (constant-value (.get receipt constant))
               (schema-owner-value (.get receipt schema-owner)))
           `((option-id . ,option-id-value)
             (source-module-id . ,source-module-id-value)
             (valid? . ,valid-value)
             (errors . ,errors-value)
             (contract-kind . ,contract-kind-value)
             (value-type-label . ,value-type-label-value)
             (required? . ,required-value)
             (optional? . ,optional-value)
             (has-default? . ,has-default-value)
             (default . ,default-value)
             (has-constant? . ,has-constant-value)
             (constant . ,constant-value)
             (schema-owner . ,schema-owner-value))))
       (.get workflow validation-receipts)))

;;; Boundary: Users can inspect option contracts without rebuilding plumbing.
;; MarlinResult <- MarlinInput
(def (UserInterfaceOptionContracts module-config)
  (user-interface-option-contracts-from-workflow
   (UserInterfaceWorkspaceWorkflow module-config)))

;;; Boundary: Policy projection fixes the module -> projection -> Rust pattern.
;; MarlinResult <- MarlinInput
(def (UserInterfacePolicyProjection module-config)
  (let* ((policy-pack (UserInterfacePolicyPack module-config))
         (pack-presentation
          (UserInterfacePolicyPackPresentation policy-pack)))
    (marlinPolicyProjection policy-pack pack-presentation)))

;;; Boundary: One receipt is the downstream handoff surface for the prefab.
;; MarlinResult <- MarlinInput
(def (UserInterfaceDeliveryReceipt module-config)
  (let* ((workflow (UserInterfaceWorkspaceWorkflow module-config))
         (evaluation (.get workflow evaluation))
         (policy-pack (UserInterfacePolicyPack module-config))
         (pack-presentation
          (UserInterfacePolicyPackPresentation policy-pack))
         (pack-catalog (UserInterfacePolicyPackCatalog policy-pack))
         (pack-catalog-presentation
          (marlinPackCatalogPresentation pack-catalog))
         (policy-projection
          (marlinPolicyProjection policy-pack pack-presentation))
         (projection-chain-receipt
          (marlinPolicyProjectionChainReceipt policy-pack pack-presentation))
         (option-contract-values
          (user-interface-option-contracts-from-workflow workflow))
         (workspace-kind-value (.get module-config kind))
         (module-system-presentation-kind-value
          (.get pack-presentation module-system-presentation-kind))
         (module-system-projection-chain-kind-value
          (.get pack-presentation module-system-projection-chain-kind))
         (policy-pack-presentation-kind-value (.get pack-presentation kind))
         (pack-catalog-kind-value (.get pack-catalog kind))
         (pack-catalog-presentation-kind-value
         (.get pack-catalog-presentation kind))
         (pack-count-value (.get pack-catalog-presentation pack-count))
         (pack-ids-value (.get pack-catalog-presentation pack-ids))
         (policy-projection-kind-value (.get policy-projection kind))
         (policy-projection-receipt-kind-value
          (.get policy-projection policy-projection-receipt-kind))
         (projection-chain-receipt-kind-value
          (.get projection-chain-receipt kind))
         (budget-receipt-kind-value
          (.get projection-chain-receipt budget-receipt-kind))
         (catalog-resolution-receipt-kind-value
          (.get projection-chain-receipt catalog-resolution-receipt-kind))
         (native-projection-payload-kind-value
          (.get policy-projection native-projection-payload-kind))
         (root-module-id-value (.get pack-presentation root-module-id))
         (root-module-kind-value (.get pack-presentation root-module-kind))
         (module-count-value (length (.get evaluation module-ids)))
         (extension-count-value (length (.get evaluation extensions)))
         (script-count-value (length (.get evaluation scripts)))
         (option-count-value (length (.get evaluation options)))
         (validation-receipt-count-value
          (length (.get workflow validation-receipts)))
         (option-contract-count-value (length option-contract-values))
         (pack-id-value (.get pack-presentation pack-id))
         (policy-object-count-value
          (.get pack-presentation policy-object-count))
         (default-policy-object-count-value
          (.get pack-presentation default-policy-object-count))
         (disabled-policy-object-count-value
          (.get pack-presentation disabled-policy-object-count))
         (policy-object-ids-value
          (.get pack-presentation policy-object-ids))
         (default-policy-object-ids-value
          (.get pack-presentation default-policy-object-ids))
         (disabled-policy-object-ids-value
          (.get pack-presentation disabled-policy-object-ids))
         (policy-families-value
          (.get pack-presentation policy-families))
         (object-operation-count-value
          (.get pack-presentation object-operation-count))
         (object-surgery-receipt-count-value
          (.get pack-presentation object-surgery-receipt-count))
         (conflict-surgery-receipt-count-value
          (.get pack-presentation conflict-surgery-receipt-count))
         (allowed-hook-ids-value
          (.get pack-presentation allowed-hook-ids))
         (import-graph-owner-value
          (.get pack-presentation import-graph-owner))
         (option-merge-owner-value
          (.get pack-presentation option-merge-owner))
         (extension-composition-owner-value
          (.get pack-presentation extension-composition-owner))
         (policy-composition-owner-value
          (.get pack-presentation policy-composition-owner))
         (native-projection-payload-owner-value
          (.get pack-presentation native-projection-payload-owner))
         (budget-receipt-owner-value
          (.get pack-presentation budget-receipt-owner))
         (catalog-resolution-receipt-owner-value
          (.get pack-presentation catalog-resolution-receipt-owner))
         (runtime-lifecycle-owner-value
          (.get pack-presentation runtime-lifecycle-owner))
         (rust-parses-scheme-source-value
          (.get pack-presentation rust-parses-scheme-source))
         (rust-handler-manufactured-value
          (.get pack-presentation rust-handler-manufactured))
         (replayable-value (.get pack-presentation replayable)))
    (.o kind: user-interface-delivery-receipt-kind
        workspace-kind: workspace-kind-value
        module-system-presentation-kind: module-system-presentation-kind-value
        module-system-projection-chain-kind:
        module-system-projection-chain-kind-value
        policy-pack-presentation-kind: policy-pack-presentation-kind-value
        pack-catalog-kind: pack-catalog-kind-value
        pack-catalog-presentation-kind: pack-catalog-presentation-kind-value
        pack-count: pack-count-value
        pack-ids: pack-ids-value
        policy-projection-kind: policy-projection-kind-value
        policy-projection-receipt-kind: policy-projection-receipt-kind-value
        policy-projection-chain-receipt-kind:
        projection-chain-receipt-kind-value
        native-projection-payload-kind: native-projection-payload-kind-value
        budget-receipt-kind: budget-receipt-kind-value
        catalog-resolution-receipt-kind:
        catalog-resolution-receipt-kind-value
        root-module-id: root-module-id-value
        root-module-kind: root-module-kind-value
        module-count: module-count-value
        extension-count: extension-count-value
        script-count: script-count-value
        option-count: option-count-value
        validation-receipt-count: validation-receipt-count-value
        option-contract-count: option-contract-count-value
        option-contracts: option-contract-values
        pack-id: pack-id-value
        policy-object-count: policy-object-count-value
        default-policy-object-count: default-policy-object-count-value
        disabled-policy-object-count: disabled-policy-object-count-value
        policy-object-ids: policy-object-ids-value
        default-policy-object-ids: default-policy-object-ids-value
        disabled-policy-object-ids: disabled-policy-object-ids-value
        policy-families: policy-families-value
        object-operation-count: object-operation-count-value
        object-surgery-receipt-count: object-surgery-receipt-count-value
        conflict-surgery-receipt-count: conflict-surgery-receipt-count-value
        allowed-hook-ids: allowed-hook-ids-value
        import-graph-owner: import-graph-owner-value
        option-merge-owner: option-merge-owner-value
        extension-composition-owner: extension-composition-owner-value
        policy-composition-owner: policy-composition-owner-value
        native-projection-payload-owner: native-projection-payload-owner-value
        budget-receipt-owner: budget-receipt-owner-value
        catalog-resolution-receipt-owner:
        catalog-resolution-receipt-owner-value
        runtime-lifecycle-owner: runtime-lifecycle-owner-value
        rust-parses-scheme-source: rust-parses-scheme-source-value
        rust-handler-manufactured: rust-handler-manufactured-value
        replayable: replayable-value
        user-entrypoints:
        '("UserInterfaceWorkspace"
          "UserInterfaceDeliveryReceipt"
          "UserInterfaceApply"
          "UserInterfacePolicyProjection"))))

;;; Boundary: Apply means build the user-facing delivery receipt.
;; MarlinResult <- MarlinInput
(def (UserInterfaceApply module-config)
  (UserInterfaceDeliveryReceipt module-config))

;;; Boundary: Thinnest user-facing entrypoint returns the delivery receipt.
;; MarlinResult <- MarlinInput
(def (UserInterface module-config)
  (UserInterfaceDeliveryReceipt module-config))
