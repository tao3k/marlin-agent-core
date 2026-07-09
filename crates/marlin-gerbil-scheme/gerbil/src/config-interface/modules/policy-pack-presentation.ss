;;; -*- Gerbil -*-
;;; Engineering note: Presentation code is the Scheme-to-Rust projection layer,
;;; not policy evaluation. The receipt fields are intentionally explicit so a
;;; failing Rust-facing packet can be traced to one Scheme projection boundary.
package: config-interface/modules

(import (only-in :clan/poo/object .get .o .ref)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-object-ref/default
                 poo-flow-module-system-owner
                 poo-flow-scheme-owner)
        :config-interface/modules/kinds
        :config-interface/modules/policy-object
        :config-interface/modules/policy-pack-core
        :config-interface/modules/policy-pack-support
        :config-interface/modules/workspace-policy
        :config-interface/modules/session-policy
        :config-interface/modules/agent-policy
        :config-interface/modules/hook-selection-policy
        :config-interface/modules/model-route-policy
        :config-interface/modules/continuation-profile-policy
        :config-interface/modules/human-review-policy
        :config-interface/modules/evidence-policy
        :config-interface/modules/failure-policy
        :config-interface/modules/memory-policy
        :config-interface/modules/domain-policy
        :config-interface/modules/catalog-projection-policy)

(export marlinPackCatalogPresentation
        marlinPolicyPackPresentation
        marlinPolicyProjection
        marlinPooLoopProgramCompilerReceipt)

;;; Boundary: Catalog presentation flattens available prefab pack facts.
;; : (-> MarlinInput MarlinResult)
(def (marlin-pack-catalog-append-field pack-presentations field-name)
  (if (null? pack-presentations)
    '()
    (apply append
           (map (lambda (pack-presentation)
                  (.ref pack-presentation field-name))
                pack-presentations))))

;;; Boundary: Catalog presentation is the user/debug listing surface.
;; MarlinResult <- MarlinInput
(def (marlinPackCatalogPresentation catalog)
  (let* ((pack-values (.get catalog packs))
         (pack-presentations (map marlinPolicyPackPresentation pack-values))
         (pack-id-values
          (map (lambda (pack-value)
                 (.get pack-value id))
               pack-values))
         (policy-object-count-value
          (apply +
                 (cons 0
                       (map (lambda (pack-presentation)
                              (.get pack-presentation policy-object-count))
                            pack-presentations))))
         (default-policy-object-count-value
          (apply +
                 (cons 0
                       (map (lambda (pack-presentation)
                              (.get pack-presentation default-policy-object-count))
                            pack-presentations))))
         (disabled-policy-object-count-value
          (apply +
                 (cons 0
                       (map (lambda (pack-presentation)
                              (.get pack-presentation disabled-policy-object-count))
                            pack-presentations))))
         (object-operation-count-value
          (apply +
                 (cons 0
                       (map (lambda (pack-presentation)
                              (.get pack-presentation object-operation-count))
                            pack-presentations))))
         (object-surgery-receipt-count-value
          (apply +
                 (cons 0
                       (map (lambda (pack-presentation)
                              (.get pack-presentation object-surgery-receipt-count))
                            pack-presentations))))
         (conflict-surgery-receipt-count-value
          (apply +
                 (cons 0
                       (map (lambda (pack-presentation)
                              (.get pack-presentation conflict-surgery-receipt-count))
                            pack-presentations)))))
    (marlin-policy-object<-alist
     (list
      (cons 'kind marlin-pack-catalog-presentation-kind)
      (cons 'catalog-kind (.get catalog kind))
      (cons 'pack-count (length pack-values))
      (cons 'pack-ids pack-id-values)
      (cons 'policy-object-count policy-object-count-value)
      (cons 'default-policy-object-count default-policy-object-count-value)
      (cons 'disabled-policy-object-count disabled-policy-object-count-value)
      (cons 'policy-families
            (marlin-pack-catalog-append-field
             pack-presentations
             'policy-families))
      (cons 'policy-object-ids
            (marlin-pack-catalog-append-field
             pack-presentations
             'policy-object-ids))
      (cons 'default-policy-object-ids
            (marlin-pack-catalog-append-field
             pack-presentations
             'default-policy-object-ids))
      (cons 'disabled-policy-object-ids
            (marlin-pack-catalog-append-field
             pack-presentations
             'disabled-policy-object-ids))
      (cons 'allowed-hook-ids
            (marlin-pack-catalog-append-field
             pack-presentations
             'allowed-hook-ids))
      (cons 'object-operation-count object-operation-count-value)
      (cons 'object-surgery-receipt-count object-surgery-receipt-count-value)
      (cons 'conflict-surgery-receipt-count conflict-surgery-receipt-count-value)
      (cons 'import-graph-owner poo-flow-module-system-owner)
      (cons 'option-merge-owner poo-flow-module-system-owner)
      (cons 'policy-composition-owner poo-flow-scheme-owner)
      (cons 'native-projection-payload-owner "rust")
      (cons 'budget-receipt-owner "rust")
      (cons 'catalog-resolution-receipt-owner "rust")
      (cons 'runtime-lifecycle-owner "rust")
      (cons 'rust-parses-scheme-source #f)
      (cons 'rust-handler-manufactured #f)
      (cons 'replayable #t)
      (cons 'user-entrypoints
            '("marlinPackCatalog"
              "marlinPackCatalogPresentation"
              "marlin-pack-catalog-find"
              "marlin-pack-catalog-root"))))))

;;; Boundary: Operation counts stay scalar for debug CLI projections.
;; MarlinResult <- MarlinInput
(def (marlin-policy-operation-count operation-values operation-name)
  (length
   (filter (lambda (operation-value)
             (string=? (.get operation-value operation) operation-name))
           operation-values)))

;;; Boundary: Matching receipt counts prove surgery actually found targets.
;; MarlinResult <- MarlinInput
(def (marlin-policy-surgery-matched-count receipt-values)
  (length
   (filter (lambda (receipt-value)
             (.get receipt-value matched?))
           receipt-values)))

;;; Boundary: Conflict reason family counts are typed projection scalars.
;; MarlinResult <- MarlinInput
(def (marlin-policy-surgery-conflict-reason-count receipt-values reason-value)
  (length
   (filter (lambda (receipt-value)
             (member reason-value (.get receipt-value conflict-reasons)))
           receipt-values)))

;;; Boundary: Disabled object counts keep object surgery auditable.
;; MarlinResult <- MarlinInput
(def (marlin-policy-disabled-object-count object-values)
  (length
   (filter (lambda (object-value)
             (and (marlin-policy-object? object-value)
                  (.get object-value policy-object-disabled)))
           object-values)))

;;; Boundary: Upstream prefab objects are policy furniture, not Rust handlers.
;; MarlinResult <- MarlinInput
;;; Boundary: The furnished default pack starts from coherent policy families.
;; MarlinResult <- MarlinInput
(def (marlin-default-policy-pack-objects)
  (list
   (marlinDefaultWorkspacePolicy)
   (marlinDefaultSessionPolicy)
   (marlinDefaultAgentPolicy)
   (marlinDefaultHookSelectionPolicy)
   (marlinDefaultModelRoutePolicy)
   (marlinDefaultContinuationProfilePolicy)
   (marlinDefaultHumanReviewPolicy)
   (marlinDefaultEvidenceGraphPolicy)
   (marlinDefaultFailureRecoveryPolicy)
   (marlinDefaultMemoryRecallPolicy)
   (marlinDefaultMemoryTriggerPolicy)
   (marlinDefaultMemoryRetentionPolicy)
   (marlinDefaultMemoryVisibilityPolicy)
   (marlinDefaultSubagentPolicy)
   (marlinDefaultContextCompressionPolicy)
   (marlinDefaultToolBatchPolicy)
   (marlinDefaultSelfEvolutionPolicy)
   (marlinDefaultCatalogProjectionPolicy)))

;;; Boundary: Default packs are furnished entrypoints over an existing module.
;; MarlinResult <- MarlinInput
(def (marlinDefaultPolicyPack module-value . maybe-pack-config)
  (let (pack-config
        (if (null? maybe-pack-config)
          (.o)
          (car maybe-pack-config)))
    (marlinPolicyPack
     (.o id:
         (poo-flow-module-object-ref/default
          pack-config
          'id
          "marlin-default-policy-pack")
         module: module-value
         policy-objects:
         (poo-flow-module-object-ref/default
          pack-config
          'policy-objects
          (marlin-default-policy-pack-objects))
         object-operations:
         (poo-flow-module-object-ref/default
          pack-config
          'object-operations
          '())
         allowed-hook-ids:
         (poo-flow-module-object-ref/default
          pack-config
          'allowed-hook-ids
          '("runtime-catalog-default-hook"))
         metadata:
         (poo-flow-module-object-ref/default
          pack-config
          'metadata
          '((owner . "marlin") (surface . "default-prefab-pack")))))))

;;; Boundary: Pack inventories are Rust-readable receipt payloads.
;; MarlinResult <- MarlinInput
(def (marlinPolicyPackInventory policy-pack)
  (let* ((policy-objects-value (.get policy-pack policy-objects))
         (default-policy-objects
          (.get policy-pack default-policy-objects))
         (allowed-hook-ids-value (.get policy-pack allowed-hook-ids))
         (policy-families-value
          (marlin-policy-object-families policy-objects-value))
         (policy-object-ids-value
          (marlin-policy-object-ids policy-objects-value))
         (default-policy-object-ids-value
          (marlin-policy-object-ids default-policy-objects))
         (disabled-policy-object-ids-value
          (marlin-policy-object-disabled-ids policy-objects-value))
         (policy-object-count-value (.get policy-pack policy-object-count))
         (default-policy-object-count-value
          (length default-policy-objects))
         (disabled-policy-object-count-value
          (.get policy-pack disabled-policy-object-count))
         (object-operation-count-value
         (.get policy-pack object-operation-count))
         (object-surgery-receipt-count-value
         (.get policy-pack object-surgery-receipt-count))
         (conflict-surgery-receipt-count-value
          (.get policy-pack conflict-surgery-receipt-count))
         (duplicate-object-conflict-count-value
          (.get policy-pack duplicate-object-conflict-count))
         (missing-target-conflict-count-value
          (.get policy-pack missing-target-conflict-count))
         (disabled-target-conflict-count-value
          (.get policy-pack disabled-target-conflict-count))
         (invalid-replacement-conflict-count-value
          (.get policy-pack invalid-replacement-conflict-count))
         (rust-parses-scheme-source-value
          (.get policy-pack rust-parses-scheme-source))
         (rust-handler-manufactured-value
          (.get policy-pack rust-handler-manufactured)))
    (marlin-policy-object<-alist
     (list
      (cons 'kind marlin-policy-pack-inventory-kind)
      (cons 'pack-kind (.get policy-pack kind))
      (cons 'pack-id (.get policy-pack id))
      (cons 'pack-owner (.get policy-pack owner))
      (cons 'pack-runtime-owner (.get policy-pack runtime-owner))
      (cons 'root-module-id (.get policy-pack root-module-id))
      (cons 'policy-object-kind marlin-policy-object-kind)
      (cons 'policy-object-count policy-object-count-value)
      (cons 'default-policy-object-count default-policy-object-count-value)
      (cons 'disabled-policy-object-count disabled-policy-object-count-value)
      (cons 'policy-families policy-families-value)
      (cons 'policy-object-ids policy-object-ids-value)
      (cons 'default-policy-object-ids default-policy-object-ids-value)
      (cons 'disabled-policy-object-ids disabled-policy-object-ids-value)
      (cons 'allowed-hook-ids allowed-hook-ids-value)
      (cons 'allowed-hook-count (length allowed-hook-ids-value))
      (cons 'object-operation-count object-operation-count-value)
      (cons 'object-surgery-receipt-count object-surgery-receipt-count-value)
      (cons 'conflict-surgery-receipt-count conflict-surgery-receipt-count-value)
      (cons 'duplicate-object-conflict-count duplicate-object-conflict-count-value)
      (cons 'missing-target-conflict-count missing-target-conflict-count-value)
      (cons 'disabled-target-conflict-count disabled-target-conflict-count-value)
      (cons 'invalid-replacement-conflict-count
            invalid-replacement-conflict-count-value)
      (cons 'replayable #t)
      (cons 'scheme-policy-owner poo-flow-scheme-owner)
      (cons 'rust-kernel-owner "rust")
      (cons 'rust-parses-scheme-source rust-parses-scheme-source-value)
      (cons 'rust-handler-manufactured rust-handler-manufactured-value)))))

;;; Boundary: Pack presentation is the stable projection pattern for Rust.
;; MarlinResult <- MarlinInput
(def (marlinPolicyPackPresentation policy-pack)
  (let* ((pack-inventory
          (marlinPolicyPackInventory policy-pack))
         (allowed-hook-ids-value (.get policy-pack allowed-hook-ids))
         (policy-object-count-value (.get policy-pack policy-object-count))
         (default-policy-object-count-value
          (.get pack-inventory default-policy-object-count))
         (disabled-policy-object-count-value
          (.get policy-pack disabled-policy-object-count))
         (policy-families-value (.get pack-inventory policy-families))
         (policy-object-ids-value (.get pack-inventory policy-object-ids))
         (default-policy-object-ids-value
          (.get pack-inventory default-policy-object-ids))
         (disabled-policy-object-ids-value
          (.get pack-inventory disabled-policy-object-ids))
         (object-operation-count-value (.get policy-pack object-operation-count))
        (object-surgery-receipt-count-value
         (.get policy-pack object-surgery-receipt-count))
         (conflict-surgery-receipt-count-value
          (.get policy-pack conflict-surgery-receipt-count))
         (duplicate-object-conflict-count-value
          (.get policy-pack duplicate-object-conflict-count))
         (missing-target-conflict-count-value
          (.get policy-pack missing-target-conflict-count))
         (disabled-target-conflict-count-value
          (.get policy-pack disabled-target-conflict-count))
         (invalid-replacement-conflict-count-value
          (.get policy-pack invalid-replacement-conflict-count))
         (add-operation-count-value (.get policy-pack add-operation-count))
         (remove-operation-count-value (.get policy-pack remove-operation-count))
         (disable-operation-count-value (.get policy-pack disable-operation-count))
         (replace-operation-count-value (.get policy-pack replace-operation-count))
         (matched-surgery-receipt-count-value
          (.get policy-pack matched-surgery-receipt-count))
         (rust-parses-scheme-source-value
          (.get policy-pack rust-parses-scheme-source))
         (rust-handler-manufactured-value
          (.get policy-pack rust-handler-manufactured)))
    (marlin-policy-object<-alist
     (list
      (cons 'kind marlin-policy-pack-presentation-kind)
      (cons 'pack-kind (.get policy-pack kind))
      (cons 'pack-id (.get policy-pack id))
      (cons 'pack-owner (.get policy-pack owner))
      (cons 'pack-runtime-owner (.get policy-pack runtime-owner))
      (cons 'pack-catalog-kind marlin-pack-catalog-kind)
      (cons 'policy-pack-inventory-kind (.get pack-inventory kind))
      (cons 'policy-facade-presentation-kind
            marlin-policy-facade-presentation-kind)
      (cons 'policy-facade-projection-chain-kind
            marlin-module-projection-chain-kind)
      (cons 'root-module-id (.get policy-pack root-module-id))
      (cons 'root-module-kind
            (marlin-policy-pack-root-module-kind policy-pack))
      (cons 'policy-object-count policy-object-count-value)
      (cons 'default-policy-object-count default-policy-object-count-value)
      (cons 'disabled-policy-object-count disabled-policy-object-count-value)
      (cons 'policy-families policy-families-value)
      (cons 'policy-object-ids policy-object-ids-value)
      (cons 'default-policy-object-ids default-policy-object-ids-value)
      (cons 'disabled-policy-object-ids disabled-policy-object-ids-value)
      (cons 'object-operation-count object-operation-count-value)
      (cons 'object-surgery-receipt-count object-surgery-receipt-count-value)
      (cons 'conflict-surgery-receipt-count conflict-surgery-receipt-count-value)
      (cons 'duplicate-object-conflict-count duplicate-object-conflict-count-value)
      (cons 'missing-target-conflict-count missing-target-conflict-count-value)
      (cons 'disabled-target-conflict-count disabled-target-conflict-count-value)
      (cons 'invalid-replacement-conflict-count
            invalid-replacement-conflict-count-value)
      (cons 'add-operation-count add-operation-count-value)
      (cons 'remove-operation-count remove-operation-count-value)
      (cons 'disable-operation-count disable-operation-count-value)
      (cons 'replace-operation-count replace-operation-count-value)
      (cons 'matched-surgery-receipt-count matched-surgery-receipt-count-value)
      (cons 'allowed-hook-ids allowed-hook-ids-value)
      (cons 'allowed-hook-count (length allowed-hook-ids-value))
      (cons 'user-entrypoints
            '("marlinPolicyPack"
              "defmarlin-policy-pack"
              "marlinDefaultPolicyPack"
              "marlinPolicyObject"
              "marlin-add-object"
              "marlin-remove-object"
              "marlin-disable-object"
              "marlin-replace-object"
              "marlinPolicyPackInventory"
              "marlinPolicyPackPresentation"
              "marlinPolicyProjection"
              "marlinPolicyProjectionReceipts"
              "marlinPolicyProjectionChainReceipt"))
      (cons 'module-evaluation-receipt-kind
            marlin-policy-pack-module-evaluation-receipt-kind)
      (cons 'projection-chain-kind marlin-module-projection-chain-kind)
      (cons 'policy-projection-receipt-kind marlin-policy-projection-kind)
      (cons 'import-graph-owner poo-flow-module-system-owner)
      (cons 'option-merge-owner poo-flow-module-system-owner)
      (cons 'extension-composition-owner poo-flow-module-system-owner)
      (cons 'policy-composition-owner poo-flow-scheme-owner)
      (cons 'native-projection-payload-owner "rust")
      (cons 'budget-receipt-owner "rust")
      (cons 'catalog-resolution-receipt-owner "rust")
      (cons 'runtime-lifecycle-owner "rust")
      (cons 'rust-parses-scheme-source rust-parses-scheme-source-value)
      (cons 'rust-handler-manufactured rust-handler-manufactured-value)
      (cons 'scheme-policy-owner poo-flow-scheme-owner)
      (cons 'rust-kernel-owner "rust")
      (cons 'replayable #t)))))

;;; Boundary: PolicyProjection<T> fixes the Scheme->Rust handoff envelope.
;; MarlinResult <- MarlinInput
(def (marlinPolicyProjection policy-pack . maybe-native-payload)
  (let* ((native-payload-input
          (if (pair? maybe-native-payload)
            (car maybe-native-payload)
            (marlinPolicyPackPresentation policy-pack)))
         (presentation
          (if (equal? (.get native-payload-input kind)
                      marlin-policy-pack-presentation-kind)
            native-payload-input
            (marlinPolicyPackPresentation policy-pack)))
         (native-payload-kind
          (.get native-payload-input kind))
         (native-payload
          (marlin-policy-object<-alist
           (list
            (cons 'kind native-payload-kind)
            (cons 'pack-id (.get policy-pack id))
            (cons 'owner (.get presentation native-projection-payload-owner))
            (cons 'payload-owner
                  (.get presentation native-projection-payload-owner))
            (cons 'policy-object-count
                  (.get presentation policy-object-count))
            (cons 'policy-families
                  (.get presentation policy-families))
            (cons 'policy-object-ids
                  (.get presentation policy-object-ids))
            (cons 'disabled-policy-object-ids
                  (.get presentation disabled-policy-object-ids))
            (cons 'allowed-hook-ids
                  (.get presentation allowed-hook-ids))
            (cons 'allowed-hook-count
                  (.get presentation allowed-hook-count))
            (cons 'object-operation-count
                  (.get presentation object-operation-count))
            (cons 'object-surgery-receipt-count
                  (.get presentation object-surgery-receipt-count))
            (cons 'conflict-surgery-receipt-count
                  (.get presentation conflict-surgery-receipt-count))
            (cons 'rust-handler-manufactured
                  (.get presentation rust-handler-manufactured))
            (cons 'replayable (.get presentation replayable))))))
    (marlin-policy-object<-alist
     (list
      (cons 'kind marlin-policy-projection-kind)
      (cons 'owner (.get presentation policy-composition-owner))
      (cons 'pack-kind (.get policy-pack kind))
      (cons 'pack-id (.get policy-pack id))
      (cons 'pack-owner (.get policy-pack owner))
      (cons 'pack-runtime-owner (.get policy-pack runtime-owner))
      (cons 'projection-chain-kind
            (.get presentation projection-chain-kind))
      (cons 'policy-facade-projection-chain-kind
            (.get presentation policy-facade-projection-chain-kind))
      (cons 'module-evaluation-receipt-kind
            (.get presentation module-evaluation-receipt-kind))
      (cons 'policy-projection-receipt-kind marlin-policy-projection-kind)
      (cons 'native-projection-payload-kind native-payload-kind)
      (cons 'native-projection-payload-owner
            (.get presentation native-projection-payload-owner))
      (cons 'native-projection-payload native-payload)
      (cons 'budget-receipt-owner
            (.get presentation budget-receipt-owner))
      (cons 'catalog-resolution-receipt-owner
            (.get presentation catalog-resolution-receipt-owner))
      (cons 'import-graph-owner
            (.get presentation import-graph-owner))
      (cons 'option-merge-owner
            (.get presentation option-merge-owner))
      (cons 'extension-composition-owner
            (.get presentation extension-composition-owner))
      (cons 'policy-composition-owner
            (.get presentation policy-composition-owner))
      (cons 'runtime-lifecycle-owner
            (.get presentation runtime-lifecycle-owner))
      (cons 'scheme-policy-owner
            (.get presentation scheme-policy-owner))
      (cons 'rust-kernel-owner
            (.get presentation rust-kernel-owner))
      (cons 'rust-parses-scheme-source
            (.get presentation rust-parses-scheme-source))
      (cons 'rust-handler-manufactured
            (.get presentation rust-handler-manufactured))
      (cons 'replayable (.get presentation replayable))))))

;;; Boundary: POO Flow compiles policy profiles into typed LoopProgram projections.
;; MarlinResult <- MarlinInput
(def (marlinPooLoopProgramCompilerReceipt profile-id-value
                                          resolved-policy-pack-value
                                          loop-program-value)
  (.o kind: marlin-poo-loop-program-compiler-receipt-kind
      profile-id: profile-id-value
      compiler-owner: "gerbil-poo-flow"
      resolved-policy-pack: resolved-policy-pack-value
      loop-program: loop-program-value
      scheme-boundary: "scheme-types-to-rust-types"
      serialization-boundary: "rust-owned-cli-trace-cross-process"))
