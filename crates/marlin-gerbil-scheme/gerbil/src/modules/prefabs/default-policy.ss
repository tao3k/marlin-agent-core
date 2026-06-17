;;; -*- Gerbil -*-
;;; Boundary: Default policy prefab delivery and catalog handoff.

package: modules/prefabs

(import (only-in :clan/poo/object .get .o)
        :modules/lib)

(export DefaultPolicyPack
        DefaultPolicyPackCatalog
        DefaultPolicyPackCatalogPresentation
        DefaultPolicyPackInventory
        DefaultPolicyPackPresentation
        DefaultPolicyProjection
        DefaultPolicyDeliveryReceipt
        DefaultPolicyApply
        default-policy-delivery-receipt-kind)

;;; Boundary: Delivery receipt kind is stable for debug/user surfaces.
;; MarlinResult <- MarlinInput
(def default-policy-delivery-receipt-kind
  "marlin.modules.prefabs.default-policy.delivery-receipt.v1")

;;; Boundary: Optional pack config is merged by Scheme/POO, never Rust.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyPack module-value . maybe-pack-config)
  (if (null? maybe-pack-config)
    (marlinDefaultPolicyPack module-value)
    (marlinDefaultPolicyPack module-value (car maybe-pack-config))))

;;; Boundary: Catalogs collect prefab packs as Scheme values.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyPackCatalog policy-pack)
  (marlinPackCatalog policy-pack))

;;; Boundary: Catalog presentation is the pack listing receipt.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyPackCatalogPresentation policy-pack)
  (marlinPackCatalogPresentation
   (DefaultPolicyPackCatalog policy-pack)))

;;; Boundary: Inventory exposes default furniture without raw pack plumbing.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyPackInventory policy-pack)
  (marlinPolicyPackInventory policy-pack))

;;; Boundary: Pack presentation is the Rust/debug scalar receipt.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyPackPresentation policy-pack)
  (marlinPolicyPackPresentation policy-pack))

;;; Boundary: Projection fixes the Scheme -> Rust handoff envelope.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyProjection module-value . maybe-pack-config)
  (let* ((policy-pack
          (if (null? maybe-pack-config)
            (DefaultPolicyPack module-value)
            (DefaultPolicyPack module-value (car maybe-pack-config))))
         (pack-presentation
          (DefaultPolicyPackPresentation policy-pack)))
    (marlinPolicyProjection policy-pack pack-presentation)))

;;; Boundary: Delivery receipt is the default pack apply result.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyDeliveryReceipt module-value . maybe-pack-config)
  (let* ((policy-pack
          (if (null? maybe-pack-config)
            (DefaultPolicyPack module-value)
            (DefaultPolicyPack module-value (car maybe-pack-config))))
         (pack-catalog (DefaultPolicyPackCatalog policy-pack))
         (pack-catalog-presentation
          (marlinPackCatalogPresentation pack-catalog))
         (pack-inventory (DefaultPolicyPackInventory policy-pack))
         (pack-presentation (DefaultPolicyPackPresentation policy-pack))
         (policy-projection
          (marlinPolicyProjection policy-pack pack-presentation))
         (projection-chain-receipt
          (marlinPolicyProjectionChainReceipt policy-pack pack-presentation))
         (pack-id-value (.get pack-presentation pack-id))
         (pack-kind-value (.get pack-presentation pack-kind))
         (pack-catalog-kind-value (.get pack-catalog kind))
         (pack-catalog-presentation-kind-value
          (.get pack-catalog-presentation kind))
         (pack-ids-value (.get pack-catalog-presentation pack-ids))
         (pack-count-value (.get pack-catalog-presentation pack-count))
         (policy-object-count-value
          (.get pack-presentation policy-object-count))
         (default-policy-object-count-value
          (.get pack-presentation default-policy-object-count))
         (disabled-policy-object-count-value
          (.get pack-presentation disabled-policy-object-count))
         (policy-families-value
          (.get pack-presentation policy-families))
         (policy-object-ids-value
          (.get pack-presentation policy-object-ids))
         (default-policy-object-ids-value
          (.get pack-presentation default-policy-object-ids))
         (disabled-policy-object-ids-value
          (.get pack-presentation disabled-policy-object-ids))
         (allowed-hook-ids-value
          (.get pack-presentation allowed-hook-ids))
         (object-operation-count-value
          (.get pack-presentation object-operation-count))
         (object-surgery-receipt-count-value
          (.get pack-presentation object-surgery-receipt-count))
         (conflict-surgery-receipt-count-value
          (.get pack-presentation conflict-surgery-receipt-count))
         (projection-kind-value (.get policy-projection kind))
         (projection-chain-kind-value
          (.get policy-projection projection-chain-kind))
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
         (inventory-kind-value (.get pack-inventory kind))
         (import-graph-owner-value
          (.get pack-presentation import-graph-owner))
         (option-merge-owner-value
          (.get pack-presentation option-merge-owner))
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
         (rust-parses-scheme-source-value
          (.get pack-presentation rust-parses-scheme-source))
         (rust-handler-manufactured-value
          (.get pack-presentation rust-handler-manufactured))
         (replayable-value (.get pack-presentation replayable)))
    (.o kind: default-policy-delivery-receipt-kind
        pack-kind: pack-kind-value
        pack-id: pack-id-value
        pack-catalog-kind: pack-catalog-kind-value
        pack-catalog-presentation-kind:
        pack-catalog-presentation-kind-value
        pack-count: pack-count-value
        pack-ids: pack-ids-value
        pack-inventory-kind: inventory-kind-value
        policy-object-count: policy-object-count-value
        default-policy-object-count: default-policy-object-count-value
        disabled-policy-object-count: disabled-policy-object-count-value
        policy-families: policy-families-value
        policy-object-ids: policy-object-ids-value
        default-policy-object-ids: default-policy-object-ids-value
        disabled-policy-object-ids: disabled-policy-object-ids-value
        allowed-hook-ids: allowed-hook-ids-value
        object-operation-count: object-operation-count-value
        object-surgery-receipt-count: object-surgery-receipt-count-value
        conflict-surgery-receipt-count: conflict-surgery-receipt-count-value
        policy-projection-kind: projection-kind-value
        projection-chain-kind: projection-chain-kind-value
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
        native-projection-payload-kind:
        native-projection-payload-kind-value
        budget-receipt-kind: budget-receipt-kind-value
        catalog-resolution-receipt-kind:
        catalog-resolution-receipt-kind-value
        import-graph-owner: import-graph-owner-value
        option-merge-owner: option-merge-owner-value
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
        '("DefaultPolicyPack"
          "DefaultPolicyPackCatalog"
          "DefaultPolicyPackCatalogPresentation"
          "DefaultPolicyDeliveryReceipt"
          "DefaultPolicyApply"
          "DefaultPolicyProjection"))))

;;; Boundary: Apply means build the user-facing delivery receipt.
;; MarlinResult <- MarlinInput
(def (DefaultPolicyApply module-value . maybe-pack-config)
  (if (null? maybe-pack-config)
    (DefaultPolicyDeliveryReceipt module-value)
    (DefaultPolicyDeliveryReceipt module-value (car maybe-pack-config))))
