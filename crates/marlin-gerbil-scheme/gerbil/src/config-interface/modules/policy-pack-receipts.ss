;;; -*- Gerbil -*-
;;; Engineering note: Receipts are replay evidence between Scheme projection and
;;; Rust validation. Field projection is explicit here because hidden fallback
;;; receipt construction would make policy-pack failures hard to audit.
package: config-interface/modules

(import (only-in :clan/poo/object .get)
        :config-interface/modules/kinds
        :config-interface/modules/policy-pack-core
        :config-interface/modules/policy-pack-presentation
        :config-interface/modules/policy-pack-support)

(export marlinPolicyModuleEvaluationReceipt
        marlinPolicyBudgetReceipt
        marlinPolicyCatalogResolutionReceipt
        marlinPolicyProjectionReceipts
        marlinPolicyProjectionChainReceipt)

;;; Boundary: Module evaluation receipt summarizes Scheme-owned composition.
;; marlinPolicyModuleEvaluationReceipt
;;   : (-> PolicyProjection ModuleEvaluationReceipt)
(def (marlinPolicyModuleEvaluationReceipt policy-projection)
  (marlin-policy-object<-alist
   (map (lambda (receipt-field)
          receipt-field)
        (list
         (cons 'kind (.get policy-projection module-evaluation-receipt-kind))
         (cons 'pack-id (.get policy-projection pack-id))
         (cons 'owner (.get policy-projection import-graph-owner))
         (cons 'import-graph-owner (.get policy-projection import-graph-owner))
         (cons 'option-merge-owner (.get policy-projection option-merge-owner))
         (cons 'extension-composition-owner
               (.get policy-projection extension-composition-owner))
         (cons 'policy-composition-owner
               (.get policy-projection policy-composition-owner))
         (cons 'replayable (.get policy-projection replayable))))))

;;; Boundary: Budget receipt is Rust-owned validation metadata.
;; marlinPolicyBudgetReceipt
;;   : (-> PolicyProjection BudgetReceipt)
(def (marlinPolicyBudgetReceipt policy-projection)
  (marlin-policy-object<-alist
   (map (lambda (receipt-field)
          receipt-field)
        (list
         (cons 'kind marlin-policy-budget-receipt-kind)
         (cons 'pack-id (.get policy-projection pack-id))
         (cons 'owner (.get policy-projection budget-receipt-owner))
         (cons 'budget-owner (.get policy-projection budget-receipt-owner))
         (cons 'runtime-lifecycle-owner
               (.get policy-projection runtime-lifecycle-owner))
         (cons 'policy-composition-owner
               (.get policy-projection policy-composition-owner))
         (cons 'replayable (.get policy-projection replayable))))))

;;; Boundary: Catalog receipt names Rust handler lookup without creating one.
;; marlinPolicyCatalogResolutionReceipt
;;   : (-> PolicyProjection CatalogResolutionReceipt)
(def (marlinPolicyCatalogResolutionReceipt policy-projection)
  (let (native-payload
        (.get policy-projection native-projection-payload))
    (marlin-policy-object<-alist
     (map (lambda (receipt-field)
            receipt-field)
          (list
           (cons 'kind marlin-policy-catalog-resolution-receipt-kind)
           (cons 'pack-id (.get policy-projection pack-id))
           (cons 'owner (.get policy-projection catalog-resolution-receipt-owner))
           (cons 'catalog-handler-lookup-owner
                 (.get policy-projection catalog-resolution-receipt-owner))
           (cons 'allowed-hook-ids (.get native-payload allowed-hook-ids))
           (cons 'allowed-hook-count (.get native-payload allowed-hook-count))
           (cons 'rust-handler-manufactured
                 (.get policy-projection rust-handler-manufactured))
           (cons 'scheme-manufactures-rust-handlers #f)
           (cons 'replayable (.get policy-projection replayable)))))))

;;; Boundary: Fixed five-family chain for prefab and custom policy packs.
;; marlin-policy-projection-receipts/direct
;;   : (-> PolicyPack NativeProjectionPayloadList PolicyProjectionChainReceipt)
(def (marlin-policy-projection-receipts/direct policy-pack . maybe-native-payload)
  (let* ((policy-projection-value
          (if (pair? maybe-native-payload)
            (marlinPolicyProjection policy-pack (car maybe-native-payload))
            (marlinPolicyProjection policy-pack)))
         (module-evaluation-receipt-value
          (marlinPolicyModuleEvaluationReceipt policy-projection-value))
         (native-payload-value
          (.get policy-projection-value native-projection-payload))
         (budget-receipt-value
          (marlinPolicyBudgetReceipt policy-projection-value))
         (catalog-resolution-receipt-value
          (marlinPolicyCatalogResolutionReceipt policy-projection-value))
         (receipt-family-id-values
          '("module_evaluation_receipt"
            "policy_projection_receipt"
            "native_projection_payload"
            "budget_receipt"
            "catalog_resolution_receipt"))
         (catalog-resolution-allowed-hook-count-value
          (.get catalog-resolution-receipt-value allowed-hook-count)))
    (marlin-policy-object<-alist
     (map (lambda (receipt-field)
            receipt-field)
          (list
           (cons 'kind marlin-policy-projection-chain-receipt-kind)
           (cons 'pack-id (.get policy-pack id))
           (cons 'receipt-family-count 5)
           (cons 'receipt-family-ids receipt-family-id-values)
           (cons 'module-evaluation-receipt module-evaluation-receipt-value)
           (cons 'policy-projection-receipt policy-projection-value)
           (cons 'native-payload native-payload-value)
           (cons 'native-projection-payload native-payload-value)
           (cons 'budget-receipt budget-receipt-value)
           (cons 'catalog-resolution-receipt catalog-resolution-receipt-value)
           (cons 'module-evaluation-receipt-owner
                 (.get module-evaluation-receipt-value owner))
           (cons 'policy-projection-receipt-owner
                 (.get policy-projection-value owner))
           (cons 'native-projection-payload-owner
                 (.get policy-projection-value native-projection-payload-owner))
           (cons 'budget-receipt-owner
                 (.get budget-receipt-value owner))
           (cons 'catalog-resolution-receipt-owner
                 (.get catalog-resolution-receipt-value owner))
           (cons 'catalog-resolution-allowed-hook-count
                 catalog-resolution-allowed-hook-count-value)
           (cons 'replayable #t))))))

;;; Boundary: Varargs public APIs share one replayable receipt application path.
;; marlin-policy-projection-receipts/apply
;;   : (-> PolicyPack NativePayloadList PolicyProjectionChainReceipt)
(def (marlin-policy-projection-receipts/apply policy-pack maybe-native-payload)
  (apply marlin-policy-projection-receipts/direct
         (map (lambda (receipt-argument)
                receipt-argument)
              (cons policy-pack maybe-native-payload))))

;;; Boundary: Public receipt helper keeps a stable varargs API.
;; marlinPolicyProjectionReceipts
;;   : (-> PolicyPack NativeProjectionPayloadList PolicyProjectionChainReceipt)
(def (marlinPolicyProjectionReceipts policy-pack . maybe-native-payload)
  (marlin-policy-projection-receipts/apply policy-pack maybe-native-payload))

;;; Boundary: Fixed receipt chain for module -> policy -> Rust validation.
;; marlinPolicyProjectionChainReceipt
;;   : (-> PolicyPack NativeProjectionPayloadList PolicyProjectionChainReceipt)
(def (marlinPolicyProjectionChainReceipt policy-pack . maybe-native-payload)
  (marlin-policy-projection-receipts/apply policy-pack maybe-native-payload))
