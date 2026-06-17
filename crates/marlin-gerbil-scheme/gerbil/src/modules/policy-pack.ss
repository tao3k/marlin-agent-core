;;; -*- Gerbil -*-
;;; Boundary: Prefab policy packs and pack projection receipts.

package: modules

(import (only-in :clan/poo/object .get .o .ref)
        :modules/kinds
        :modules/core
        :modules/policy-object
        :modules/workspace-policy
        :modules/session-policy
        :modules/agent-policy
        :modules/hook-selection-policy
        :modules/model-route-policy
        :modules/continuation-profile-policy
        :modules/human-review-policy
        :modules/evidence-policy
        :modules/failure-policy
        :modules/memory-policy
        :modules/catalog-projection-policy
        :modules/evaluation)

(export marlinPolicyPack
        defmarlin-policy-pack
        marlinDefaultPolicyPack
        marlinPackCatalog
        marlin-pack-catalog-find
        marlin-pack-catalog-root
        marlinPackCatalogPresentation
        marlinPolicyPackInventory
        marlinPolicyPackPresentation
        marlinPolicyProjection
        marlinPolicyProjectionChainReceipt)

;;; Boundary: Policy packs are upstream prefab bundles over POO modules.
;; MarlinResult <- MarlinInput
(def (marlinPolicyPack pack-config)
  (let* ((module-value
          (marlin-module-object-ref/default pack-config 'module #f))
         (catalog-value
          (marlin-module-object-ref/default
           pack-config
           'catalog
           (if module-value
             (marlinModuleCatalog module-value)
             #f)))
         (root-module-id-value
          (marlin-module-object-ref/default
           pack-config
           'root-module-id
           (if module-value
             (.get module-value id)
             #f)))
         (default-policy-objects-value
          (marlin-module-object-ref/default
           pack-config
           'policy-objects
           '()))
         (object-operations-value
          (marlin-module-object-ref/default
           pack-config
           'object-operations
           '()))
         (operation-result
          (marlin-policy-pack-apply-operations
           default-policy-objects-value
           object-operations-value))
         (surgery-receipts-value
          (.get operation-result surgery-receipts)))
    (.o kind: marlin-policy-pack-kind
        id:
        (marlin-module-object-ref/default
         pack-config
         'id
         "anonymous-marlin-policy-pack")
        module: module-value
        catalog: catalog-value
        root-module-id: root-module-id-value
        allowed-hook-ids:
        (marlin-module-object-ref/default
         pack-config
         'allowed-hook-ids
         '())
        default-policy-objects: default-policy-objects-value
        policy-objects: (.get operation-result policy-objects)
        object-operations: object-operations-value
        object-surgery-receipts: surgery-receipts-value
        policy-object-count:
        (length (.get operation-result policy-objects))
        object-operation-count:
        (+ (.get operation-result add-operation-count)
           (.get operation-result remove-operation-count)
           (.get operation-result disable-operation-count)
           (.get operation-result replace-operation-count))
        object-surgery-receipt-count:
        (+ (.get operation-result add-operation-count)
           (.get operation-result remove-operation-count)
           (.get operation-result disable-operation-count)
           (.get operation-result replace-operation-count))
        disabled-policy-object-count:
        (marlin-policy-disabled-object-count
         (.get operation-result policy-objects))
        add-operation-count: (.get operation-result add-operation-count)
        remove-operation-count: (.get operation-result remove-operation-count)
        disable-operation-count: (.get operation-result disable-operation-count)
        replace-operation-count: (.get operation-result replace-operation-count)
        matched-surgery-receipt-count:
        (.get operation-result matched-surgery-receipt-count)
        conflict-surgery-receipt-count:
        (.get operation-result conflict-surgery-receipt-count)
        duplicate-object-conflict-count:
        (marlin-policy-surgery-conflict-reason-count
         surgery-receipts-value
         "duplicate-object")
        missing-target-conflict-count:
        (marlin-policy-surgery-conflict-reason-count
         surgery-receipts-value
         "missing-target")
        disabled-target-conflict-count:
        (marlin-policy-surgery-conflict-reason-count
         surgery-receipts-value
         "disabled-target")
        invalid-replacement-conflict-count:
        (marlin-policy-surgery-conflict-reason-count
         surgery-receipts-value
         "invalid-replacement")
        metadata:
        (marlin-module-object-ref/default
         pack-config
         'metadata
         '())
        owner: "gerbil-poo"
        runtime-owner: "rust"
        rust-parses-scheme-source: #f
        rust-handler-manufactured: #f)))

;;; Boundary: Level-1 prefab API exposes object surgery without plumbing.
;; MarlinResult <- MarlinInput
(defrules defmarlin-policy-pack ()
  ((_ binding
      (id pack-id)
      (module module-value)
      (policy-objects object-value ...)
      (object-operations operation-value ...)
      (allowed-hook-ids allowed-hook-id-value ...)
      (metadata metadata-value))
   (def binding
     (marlinPolicyPack
      (.o id: pack-id
          module: module-value
          policy-objects: (list object-value ...)
          object-operations: (list operation-value ...)
          allowed-hook-ids: (list allowed-hook-id-value ...)
          metadata: metadata-value))))
  ((_ binding
      (id pack-id)
      (module module-value)
      (policy-objects object-value ...)
      (object-operations operation-value ...))
   (def binding
     (marlinPolicyPack
      (.o id: pack-id
          module: module-value
          policy-objects: (list object-value ...)
          object-operations: (list operation-value ...)
          allowed-hook-ids: '()
          metadata: '())))))

;;; Boundary: Pack catalogs keep prefab bundles first-class.
;; MarlinResult <- MarlinInput
(def (marlinPackCatalog . pack-values)
  (.o kind: marlin-pack-catalog-kind
      packs: pack-values))

;;; Boundary: Pack lookup is explicit and deterministic.
;; MarlinResult <- MarlinInput
(def (marlin-pack-catalog-find catalog pack-id-value)
  (let (matches
        (filter (lambda (pack)
                  (string=? (.get pack id) pack-id-value))
                (.get catalog packs)))
    (if (pair? matches)
      (car matches)
      #f)))

;;; Boundary: A missing pack id means the first catalog pack is the root.
;; MarlinResult <- MarlinInput
(def (marlin-pack-catalog-root catalog pack-id-value)
  (cond
   (pack-id-value
    (or (marlin-pack-catalog-find catalog pack-id-value)
        (error "marlin policy pack root not found" pack-id-value)))
   ((pair? (.get catalog packs))
    (car (.get catalog packs)))
   (else
    (error "marlin policy pack catalog is empty"))))

;;; Boundary: Catalog presentation flattens available prefab pack facts.
;; MarlinResult <- MarlinInput
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
    (.o kind: marlin-pack-catalog-presentation-kind
        catalog-kind: (.get catalog kind)
        pack-count: (length pack-values)
        pack-ids: pack-id-values
        policy-object-count: policy-object-count-value
        default-policy-object-count: default-policy-object-count-value
        disabled-policy-object-count: disabled-policy-object-count-value
        policy-families:
        (marlin-pack-catalog-append-field
         pack-presentations
         'policy-families)
        policy-object-ids:
        (marlin-pack-catalog-append-field
         pack-presentations
         'policy-object-ids)
        default-policy-object-ids:
        (marlin-pack-catalog-append-field
         pack-presentations
         'default-policy-object-ids)
        disabled-policy-object-ids:
        (marlin-pack-catalog-append-field
         pack-presentations
         'disabled-policy-object-ids)
        allowed-hook-ids:
        (marlin-pack-catalog-append-field
         pack-presentations
         'allowed-hook-ids)
        object-operation-count: object-operation-count-value
        object-surgery-receipt-count: object-surgery-receipt-count-value
        conflict-surgery-receipt-count: conflict-surgery-receipt-count-value
        import-graph-owner: "gerbil-module-system"
        option-merge-owner: "gerbil-poo"
        policy-composition-owner: "gerbil-poo"
        native-projection-payload-owner: "rust"
        budget-receipt-owner: "rust"
        catalog-resolution-receipt-owner: "rust"
        runtime-lifecycle-owner: "rust"
        rust-parses-scheme-source: #f
        rust-handler-manufactured: #f
        replayable: #t
        user-entrypoints:
        '("marlinPackCatalog"
          "marlinPackCatalogPresentation"
          "marlin-pack-catalog-find"
          "marlin-pack-catalog-root"))))

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
   (marlinDefaultMemoryTriggerPolicy)
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
         (marlin-module-object-ref/default
          pack-config
          'id
          "marlin-default-policy-pack")
         module: module-value
         policy-objects:
         (marlin-module-object-ref/default
          pack-config
          'policy-objects
          (marlin-default-policy-pack-objects))
         object-operations:
         (marlin-module-object-ref/default
          pack-config
          'object-operations
          '())
         allowed-hook-ids:
         (marlin-module-object-ref/default
          pack-config
          'allowed-hook-ids
          '("runtime-catalog-default-hook"))
         metadata:
         (marlin-module-object-ref/default
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
    (.o kind: marlin-policy-pack-inventory-kind
        pack-kind: (.get policy-pack kind)
        pack-id: (.get policy-pack id)
        pack-owner: (.get policy-pack owner)
        pack-runtime-owner: (.get policy-pack runtime-owner)
        root-module-id: (.get policy-pack root-module-id)
        policy-object-kind: marlin-policy-object-kind
        policy-object-count: policy-object-count-value
        default-policy-object-count: default-policy-object-count-value
        disabled-policy-object-count: disabled-policy-object-count-value
        policy-families: policy-families-value
        policy-object-ids: policy-object-ids-value
        default-policy-object-ids: default-policy-object-ids-value
        disabled-policy-object-ids: disabled-policy-object-ids-value
        allowed-hook-ids: allowed-hook-ids-value
        allowed-hook-count: (length allowed-hook-ids-value)
        object-operation-count: object-operation-count-value
        object-surgery-receipt-count: object-surgery-receipt-count-value
        conflict-surgery-receipt-count: conflict-surgery-receipt-count-value
        duplicate-object-conflict-count: duplicate-object-conflict-count-value
        missing-target-conflict-count: missing-target-conflict-count-value
        disabled-target-conflict-count: disabled-target-conflict-count-value
        invalid-replacement-conflict-count:
        invalid-replacement-conflict-count-value
        replayable: #t
        scheme-policy-owner: "gerbil-poo"
        rust-kernel-owner: "rust"
        rust-parses-scheme-source: rust-parses-scheme-source-value
        rust-handler-manufactured: rust-handler-manufactured-value)))

;;; Boundary: Pack presentation is the stable projection pattern for Rust.
;; MarlinResult <- MarlinInput
(def (marlinPolicyPackPresentation policy-pack)
  (let* ((module-system-presentation
          (marlinModuleSystemPresentation
           (.get policy-pack catalog)
           (.get policy-pack root-module-id)
           (.get policy-pack allowed-hook-ids)))
         (pack-inventory
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
    (.o kind: marlin-policy-pack-presentation-kind
        pack-kind: (.get policy-pack kind)
        pack-id: (.get policy-pack id)
        pack-owner: (.get policy-pack owner)
        pack-runtime-owner: (.get policy-pack runtime-owner)
        pack-catalog-kind: marlin-pack-catalog-kind
        policy-pack-inventory-kind: (.get pack-inventory kind)
        module-system-presentation-kind:
        (.get module-system-presentation kind)
        module-system-projection-chain-kind:
        (.get module-system-presentation projection-chain-kind)
        root-module-id: (.get policy-pack root-module-id)
        root-module-kind: (.get module-system-presentation root-module-kind)
        policy-object-count: policy-object-count-value
        default-policy-object-count: default-policy-object-count-value
        disabled-policy-object-count: disabled-policy-object-count-value
        policy-families: policy-families-value
        policy-object-ids: policy-object-ids-value
        default-policy-object-ids: default-policy-object-ids-value
        disabled-policy-object-ids: disabled-policy-object-ids-value
        object-operation-count: object-operation-count-value
        object-surgery-receipt-count: object-surgery-receipt-count-value
        conflict-surgery-receipt-count: conflict-surgery-receipt-count-value
        duplicate-object-conflict-count: duplicate-object-conflict-count-value
        missing-target-conflict-count: missing-target-conflict-count-value
        disabled-target-conflict-count: disabled-target-conflict-count-value
        invalid-replacement-conflict-count:
        invalid-replacement-conflict-count-value
        add-operation-count: add-operation-count-value
        remove-operation-count: remove-operation-count-value
        disable-operation-count: disable-operation-count-value
        replace-operation-count: replace-operation-count-value
        matched-surgery-receipt-count: matched-surgery-receipt-count-value
        allowed-hook-ids: allowed-hook-ids-value
        allowed-hook-count: (length allowed-hook-ids-value)
        user-entrypoints:
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
          "marlinPolicyProjectionChainReceipt")
        module-evaluation-receipt-kind:
        (.get module-system-presentation module-evaluation-receipt-kind)
        projection-chain-kind:
        (.get module-system-presentation projection-chain-kind)
        policy-projection-receipt-kind: marlin-policy-projection-kind
        import-graph-owner:
        (.get module-system-presentation import-graph-owner)
        option-merge-owner:
        (.get module-system-presentation option-merge-owner)
        extension-composition-owner:
        (.get module-system-presentation extension-composition-owner)
        policy-composition-owner: "gerbil-poo"
        native-projection-payload-owner:
        (.get module-system-presentation native-projection-payload-owner)
        budget-receipt-owner:
        (.get module-system-presentation budget-receipt-owner)
        catalog-resolution-receipt-owner:
        (.get module-system-presentation catalog-resolution-receipt-owner)
        runtime-lifecycle-owner:
        (.get module-system-presentation runtime-lifecycle-owner)
        rust-parses-scheme-source:
        rust-parses-scheme-source-value
        rust-handler-manufactured: rust-handler-manufactured-value
        scheme-policy-owner: "gerbil-poo"
        rust-kernel-owner: "rust"
        replayable: #t)))

;;; Boundary: PolicyProjection<T> fixes the Scheme->Rust handoff envelope.
;; MarlinResult <- MarlinInput
(def (marlinPolicyProjection policy-pack . maybe-native-payload)
  (let* ((native-payload
          (if (pair? maybe-native-payload)
            (car maybe-native-payload)
            (marlinPolicyPackPresentation policy-pack)))
         (presentation
          (if (equal? (.get native-payload kind)
                      marlin-policy-pack-presentation-kind)
            native-payload
            (marlinPolicyPackPresentation policy-pack))))
    (.o kind: marlin-policy-projection-kind
        pack-kind: (.get policy-pack kind)
        pack-id: (.get policy-pack id)
        pack-owner: (.get policy-pack owner)
        pack-runtime-owner: (.get policy-pack runtime-owner)
        projection-chain-kind:
        (.get presentation projection-chain-kind)
        module-system-projection-chain-kind:
        (.get presentation module-system-projection-chain-kind)
        module-evaluation-receipt-kind:
        (.get presentation module-evaluation-receipt-kind)
        policy-projection-receipt-kind: marlin-policy-projection-kind
        native-projection-payload-kind:
        (.get native-payload kind)
        native-projection-payload-owner:
        (.get presentation native-projection-payload-owner)
        native-projection-payload: native-payload
        budget-receipt-owner:
        (.get presentation budget-receipt-owner)
        catalog-resolution-receipt-owner:
        (.get presentation catalog-resolution-receipt-owner)
        import-graph-owner:
        (.get presentation import-graph-owner)
        option-merge-owner:
        (.get presentation option-merge-owner)
        extension-composition-owner:
        (.get presentation extension-composition-owner)
        policy-composition-owner:
        (.get presentation policy-composition-owner)
        runtime-lifecycle-owner:
        (.get presentation runtime-lifecycle-owner)
        scheme-policy-owner:
        (.get presentation scheme-policy-owner)
        rust-kernel-owner:
        (.get presentation rust-kernel-owner)
        rust-parses-scheme-source:
        (.get presentation rust-parses-scheme-source)
        rust-handler-manufactured:
        (.get presentation rust-handler-manufactured)
        replayable: (.get presentation replayable))))

;;; Boundary: Fixed receipt chain for module -> policy -> Rust validation.
;; MarlinResult <- MarlinInput
(def (marlinPolicyProjectionChainReceipt policy-pack . maybe-native-payload)
  (let* ((policy-projection
          (if (pair? maybe-native-payload)
            (marlinPolicyProjection policy-pack (car maybe-native-payload))
            (marlinPolicyProjection policy-pack)))
         (native-payload
          (.get policy-projection native-projection-payload)))
    (.o kind: marlin-policy-projection-chain-receipt-kind
        pack-id: (.get policy-projection pack-id)
        pack-kind: (.get policy-projection pack-kind)
        module-evaluation-receipt-kind:
        (.get policy-projection module-evaluation-receipt-kind)
        policy-projection-receipt-kind:
        (.get policy-projection policy-projection-receipt-kind)
        policy-projection-receipt: policy-projection
        native-projection-payload-kind:
        (.get policy-projection native-projection-payload-kind)
        native-projection-payload-owner:
        (.get policy-projection native-projection-payload-owner)
        native-projection-payload: native-payload
        budget-receipt-kind: marlin-policy-budget-receipt-kind
        budget-receipt-owner:
        (.get policy-projection budget-receipt-owner)
        catalog-resolution-receipt-kind:
        marlin-policy-catalog-resolution-receipt-kind
        catalog-resolution-receipt-owner:
        (.get policy-projection catalog-resolution-receipt-owner)
        runtime-lifecycle-owner:
        (.get policy-projection runtime-lifecycle-owner)
        import-graph-owner:
        (.get policy-projection import-graph-owner)
        option-merge-owner:
        (.get policy-projection option-merge-owner)
        extension-composition-owner:
        (.get policy-projection extension-composition-owner)
        policy-composition-owner:
        (.get policy-projection policy-composition-owner)
        scheme-policy-owner:
        (.get policy-projection scheme-policy-owner)
        rust-kernel-owner:
        (.get policy-projection rust-kernel-owner)
        rust-parses-scheme-source:
        (.get policy-projection rust-parses-scheme-source)
        rust-handler-manufactured:
        (.get policy-projection rust-handler-manufactured)
        replayable: (.get policy-projection replayable))))
