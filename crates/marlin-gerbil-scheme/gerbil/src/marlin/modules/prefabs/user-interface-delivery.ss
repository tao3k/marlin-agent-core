;;; -*- Gerbil -*-
;;; Boundary: User-interface delivery receipt and Rust projection handoff.

package: marlin/modules/prefabs

(import (only-in :clan/poo/object .get .o)
        :marlin/modules/lib
        :marlin/modules/prefabs/user-interface)

(export UserInterface
        UserInterfaceWorkspace
        UserInterfaceLoopGovernorPattern
        UserInterfaceLoopGovernorStrategy
        UserInterfaceLoopGovernor
        UserInterfaceLoopGovernorStateFacts
        UserInterfaceLoopGovernorRequestEnvelope
        UserInterfaceLoopGovernorL1Receipt
        UserInterfaceLoopGovernorRuntimeManifest
        UserInterfaceDeliveryReceipt
        UserInterfaceApply
        UserInterfacePolicyProjection
        UserInterfaceOptionContracts
        user-interface-delivery-receipt-kind)

;;; Boundary: Delivery receipt kind is stable for user/debug surfaces.
;; MarlinResult <- MarlinInput
(def user-interface-delivery-receipt-kind
  "marlin.modules.prefabs.user-interface.delivery-receipt.v1")

;;; Boundary: Delivery receipt probes alist projections without taking ownership.
;; MarlinResult <- MarlinInput Symbol MarlinInput
(def (user-interface-delivery-alist-ref alist key default)
  (let (found (and (list? alist) (assoc key alist)))
    (if found (cdr found) default)))

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
(def (UserInterfaceOptionContracts workspace-config)
  (user-interface-option-contracts-from-workflow
   (UserInterfaceWorkspaceWorkflow workspace-config)))

;;; Boundary: Policy projection fixes the module -> projection -> Rust pattern.
;; MarlinResult <- MarlinInput
(def (UserInterfacePolicyProjection workspace-config)
  (let* ((policy-pack (UserInterfacePolicyPack workspace-config))
         (pack-presentation
          (UserInterfacePolicyPackPresentation policy-pack)))
    (marlinPolicyProjection policy-pack pack-presentation)))

;;; Boundary: One receipt is the downstream handoff surface for the prefab.
;; MarlinResult <- MarlinInput
(def (UserInterfaceDeliveryReceipt workspace-config)
  (let* ((workflow (UserInterfaceWorkspaceWorkflow workspace-config))
         (evaluation (.get workflow evaluation))
         (policy-pack (UserInterfacePolicyPack workspace-config))
         (pack-presentation
          (UserInterfacePolicyPackPresentation policy-pack))
         (pack-catalog (UserInterfacePolicyPackCatalog policy-pack))
         (pack-catalog-presentation
          (marlinPackCatalogPresentation pack-catalog))
         (policy-projection
          (marlinPolicyProjection policy-pack pack-presentation))
         (projection-chain-receipt
          (marlinPolicyProjectionChainReceipt policy-pack pack-presentation))
         (loop-governor-runtime-manifest
          (UserInterfaceLoopGovernorRuntimeManifest workspace-config))
         (loop-governor-request-envelope
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'request-envelope
           '()))
         (loop-governor-abi-manifest
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'abi-manifest
           '()))
         (loop-governor-l1-receipt
          (UserInterfaceLoopGovernorL1Receipt workspace-config))
         (marlin-loops-policy
          (UserInterfaceMarlinLoopsPolicy workspace-config))
         (option-contract-values
          (user-interface-option-contracts-from-workflow workflow))
         (workspace-kind-value (.get workspace-config kind))
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
         (projection-receipt-family-count-value
         (.get projection-chain-receipt receipt-family-count))
         (projection-receipt-family-ids-value
          (.get projection-chain-receipt receipt-family-ids))
         (module-evaluation-receipt-owner-value
          (.get projection-chain-receipt module-evaluation-receipt-owner))
         (policy-projection-receipt-owner-value
          (.get projection-chain-receipt policy-projection-receipt-owner))
         (catalog-resolution-allowed-hook-count-value
          (.get projection-chain-receipt catalog-resolution-allowed-hook-count))
         (budget-receipt-kind-value
          marlin-policy-budget-receipt-kind)
         (catalog-resolution-receipt-kind-value
          marlin-policy-catalog-resolution-receipt-kind)
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
          (.get projection-chain-receipt native-projection-payload-owner))
         (budget-receipt-owner-value
          (.get projection-chain-receipt budget-receipt-owner))
         (catalog-resolution-receipt-owner-value
          (.get projection-chain-receipt catalog-resolution-receipt-owner))
         (runtime-lifecycle-owner-value
          (.get pack-presentation runtime-lifecycle-owner))
         (loop-control-plane-runtime-manifest-schema-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'schema
           #f))
         (loop-control-plane-request-schema-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'request-schema
           #f))
         (loop-control-plane-receipt-schema-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'receipt-schema
           #f))
         (loop-control-plane-abi-schema-value
          (user-interface-delivery-alist-ref
           loop-governor-abi-manifest
           'schema
           #f))
         (loop-control-plane-operation-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'operation
           #f))
         (loop-control-plane-target-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'target
           #f))
         (loop-control-plane-transport-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'transport
           #f))
         (loop-control-plane-control-owner-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'control-owner
           #f))
         (loop-control-plane-execution-owner-value
          (user-interface-delivery-alist-ref
           loop-governor-runtime-manifest
           'execution-owner
           #f))
         (loop-control-plane-open-patterns-value
          (user-interface-delivery-alist-ref
           loop-governor-request-envelope
           'open-patterns
           '()))
         (loop-control-plane-blocked-patterns-value
          (user-interface-delivery-alist-ref
           loop-governor-request-envelope
           'blocked-patterns
           '()))
         (loop-control-plane-status-value
          (user-interface-delivery-alist-ref
           loop-governor-l1-receipt
           'status
           #f))
         (marlin-loops-policy-kind-value
          (.get marlin-loops-policy kind))
         (marlin-loops-policy-id-value
          (.get marlin-loops-policy id))
         (marlin-loops-policy-owner-value
          (.get marlin-loops-policy owner))
         (marlin-loops-policy-source-value
          (.get marlin-loops-policy source))
         (marlin-loops-policy-control-plane-owner-value
          (.get marlin-loops-policy control-plane-owner))
         (marlin-loops-policy-runtime-execution-owner-value
         (.get marlin-loops-policy runtime-execution-owner))
         (marlin-loops-policy-receipt-contracts-value
          (.get marlin-loops-policy receipt-contracts))
         (marlin-loops-policy-receipt-family-ids-value
          (.get marlin-loops-policy receipt-family-ids))
         (marlin-loops-policy-receipt-schema-ids-value
          (.get marlin-loops-policy receipt-schema-ids))
         (marlin-loops-policy-receipt-contract-owners-value
          (foldl (lambda (contract owners)
                   (let (owner-value (cdr (assq 'owner contract)))
                     (if (member owner-value owners)
                       owners
                       (append owners (list owner-value)))))
                 '()
                 marlin-loops-policy-receipt-contracts-value))
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
        projection-receipt-family-count:
        projection-receipt-family-count-value
        projection-receipt-family-ids:
        projection-receipt-family-ids-value
        module-evaluation-receipt-owner:
        module-evaluation-receipt-owner-value
        policy-projection-receipt-owner:
        policy-projection-receipt-owner-value
        catalog-resolution-allowed-hook-count:
        catalog-resolution-allowed-hook-count-value
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
        loop-control-plane-owner: "poo-flow"
        loop-control-plane-runtime-manifest-schema:
        loop-control-plane-runtime-manifest-schema-value
        loop-control-plane-request-schema:
        loop-control-plane-request-schema-value
        loop-control-plane-receipt-schema:
        loop-control-plane-receipt-schema-value
        loop-control-plane-abi-schema:
        loop-control-plane-abi-schema-value
        loop-control-plane-operation: loop-control-plane-operation-value
        loop-control-plane-target: loop-control-plane-target-value
        loop-control-plane-transport: loop-control-plane-transport-value
        loop-control-plane-control-owner:
        loop-control-plane-control-owner-value
        loop-control-plane-execution-owner:
        loop-control-plane-execution-owner-value
        loop-control-plane-open-patterns:
        loop-control-plane-open-patterns-value
        loop-control-plane-blocked-patterns:
        loop-control-plane-blocked-patterns-value
        loop-control-plane-status: loop-control-plane-status-value
        marlin-loops-policy-kind: marlin-loops-policy-kind-value
        marlin-loops-policy-id: marlin-loops-policy-id-value
        marlin-loops-policy-owner: marlin-loops-policy-owner-value
        marlin-loops-policy-source: marlin-loops-policy-source-value
        marlin-loops-policy-control-plane-owner:
        marlin-loops-policy-control-plane-owner-value
        marlin-loops-policy-runtime-execution-owner:
        marlin-loops-policy-runtime-execution-owner-value
        marlin-loops-policy-receipt-family-count:
        (length marlin-loops-policy-receipt-family-ids-value)
        marlin-loops-policy-receipt-family-ids:
        marlin-loops-policy-receipt-family-ids-value
        marlin-loops-policy-receipt-contract-count:
        (length marlin-loops-policy-receipt-contracts-value)
        marlin-loops-policy-receipt-contracts:
        marlin-loops-policy-receipt-contracts-value
        marlin-loops-policy-receipt-schema-ids:
        marlin-loops-policy-receipt-schema-ids-value
        marlin-loops-policy-receipt-contract-owners:
        marlin-loops-policy-receipt-contract-owners-value
        rust-parses-scheme-source: rust-parses-scheme-source-value
        rust-handler-manufactured: rust-handler-manufactured-value
        replayable: replayable-value
        user-entrypoints:
        '("UserInterfaceWorkspace"
          "UserInterfaceLoopGovernorRuntimeManifest"
          "UserInterfaceDeliveryReceipt"
          "UserInterfaceApply"
          "UserInterfacePolicyProjection"))))

;;; Boundary: Apply means build the user-facing delivery receipt.
;; MarlinResult <- MarlinInput
(def (UserInterfaceApply workspace-config)
  (UserInterfaceDeliveryReceipt workspace-config))

;;; Boundary: Thinnest user-facing entrypoint returns the delivery receipt.
;; MarlinResult <- MarlinInput
(def (UserInterface workspace-config)
  (UserInterfaceDeliveryReceipt workspace-config))
