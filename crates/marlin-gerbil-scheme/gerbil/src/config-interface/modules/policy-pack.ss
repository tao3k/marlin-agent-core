;;; -*- Gerbil -*-
;;; Boundary: Prefab policy packs and pack projection receipts.

package: config-interface/modules

(import (only-in :clan/poo/object .get .o .ref)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-object-has-slot?
                 poo-flow-module-object-ref/default
                 poo-flow-module-name
                 poo-flow-module-system-owner
                 poo-flow-scheme-owner)
        :config-interface/modules/kinds
        :config-interface/modules/policy-object
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
        marlinPooLoopProgramCompilerReceipt
        marlinRealRepair001ResolvedPolicyPack
        marlinRealRepair001LoopProgram
        marlinRealRepair001LoopProgramCompilerReceipt
        marlinPolicyCombinationMatrixResolvedPolicyPack
        marlinPolicyCombinationMatrixLoopProgram
        marlinPolicyCombinationMatrixLoopProgramCompilerReceipt
        marlinPolicyModuleEvaluationReceipt
        marlinPolicyBudgetReceipt
        marlinPolicyCatalogResolutionReceipt
        marlinPolicyProjectionReceipts
        marlinPolicyProjectionChainReceipt)

;;; Boundary: Policy packs build catalog metadata without importing evalModules.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-module-catalog module-value)
  (.o kind: marlin-module-catalog-kind
      modules: (list module-value)))

;;; Boundary: Pack presentations summarize module metadata without evalModules.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-root-module-kind policy-pack)
  (let (module-value (.get policy-pack module))
    (if module-value
      (.get module-value kind)
      #f)))

;;; Boundary: policy packs accept raw poo-flow module descriptors.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-module-id module-value)
  (cond
   ((not module-value) #f)
   ((poo-flow-module-object-has-slot? module-value 'id)
    (.get module-value id))
   (else
    (poo-flow-module-name module-value))))

;;; Boundary: Policy pack projection keeps runtime evaluation out of facade load.
;; MarlinResult <- MarlinInput
(def marlin-policy-pack-module-evaluation-receipt-kind
  "marlin.config-interface.policy-pack.module-evaluation-receipt.v1")

;;; Boundary: Policy packs are upstream prefab bundles over POO modules.
;; MarlinResult <- MarlinInput
(def (marlinPolicyPack pack-config)
  (let* ((module-value
          (poo-flow-module-object-ref/default pack-config 'module #f))
         (catalog-value
          (poo-flow-module-object-ref/default
           pack-config
           'catalog
           (if module-value
             (marlin-policy-pack-module-catalog module-value)
             #f)))
         (root-module-id-value
         (poo-flow-module-object-ref/default
           pack-config
           'root-module-id
           (if module-value
             (marlin-policy-pack-module-id module-value)
             #f)))
         (default-policy-objects-value
          (poo-flow-module-object-ref/default
           pack-config
           'policy-objects
           '()))
         (object-operations-value
          (poo-flow-module-object-ref/default
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
        (poo-flow-module-object-ref/default
         pack-config
         'id
         "anonymous-marlin-policy-pack")
        module: module-value
        catalog: catalog-value
        root-module-id: root-module-id-value
        allowed-hook-ids:
        (poo-flow-module-object-ref/default
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
        (poo-flow-module-object-ref/default
         pack-config
         'metadata
         '())
        owner: poo-flow-scheme-owner
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
        import-graph-owner: poo-flow-module-system-owner
        option-merge-owner: poo-flow-module-system-owner
        policy-composition-owner: poo-flow-scheme-owner
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
        scheme-policy-owner: poo-flow-scheme-owner
        rust-kernel-owner: "rust"
        rust-parses-scheme-source: rust-parses-scheme-source-value
        rust-handler-manufactured: rust-handler-manufactured-value)))

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
    (.o kind: marlin-policy-pack-presentation-kind
        pack-kind: (.get policy-pack kind)
        pack-id: (.get policy-pack id)
        pack-owner: (.get policy-pack owner)
        pack-runtime-owner: (.get policy-pack runtime-owner)
        pack-catalog-kind: marlin-pack-catalog-kind
        policy-pack-inventory-kind: (.get pack-inventory kind)
        policy-facade-presentation-kind:
        marlin-policy-facade-presentation-kind
        policy-facade-projection-chain-kind:
        marlin-module-projection-chain-kind
        root-module-id: (.get policy-pack root-module-id)
        root-module-kind:
        (marlin-policy-pack-root-module-kind policy-pack)
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
          "marlinPolicyProjectionReceipts"
          "marlinPolicyProjectionChainReceipt")
        module-evaluation-receipt-kind:
        marlin-policy-pack-module-evaluation-receipt-kind
        projection-chain-kind:
        marlin-module-projection-chain-kind
        policy-projection-receipt-kind: marlin-policy-projection-kind
        import-graph-owner: poo-flow-module-system-owner
        option-merge-owner: poo-flow-module-system-owner
        extension-composition-owner: poo-flow-module-system-owner
        policy-composition-owner: poo-flow-scheme-owner
        native-projection-payload-owner: "rust"
        budget-receipt-owner: "rust"
        catalog-resolution-receipt-owner: "rust"
        runtime-lifecycle-owner: "rust"
        rust-parses-scheme-source:
        rust-parses-scheme-source-value
        rust-handler-manufactured: rust-handler-manufactured-value
        scheme-policy-owner: poo-flow-scheme-owner
        rust-kernel-owner: "rust"
        replayable: #t)))

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
          (.o kind: native-payload-kind
              pack-id: (.get policy-pack id)
              owner: (.get presentation native-projection-payload-owner)
              payload-owner:
              (.get presentation native-projection-payload-owner)
              policy-object-count:
              (.get presentation policy-object-count)
              policy-families:
              (.get presentation policy-families)
              policy-object-ids:
              (.get presentation policy-object-ids)
              disabled-policy-object-ids:
              (.get presentation disabled-policy-object-ids)
              allowed-hook-ids:
              (.get presentation allowed-hook-ids)
              allowed-hook-count:
              (.get presentation allowed-hook-count)
              object-operation-count:
              (.get presentation object-operation-count)
              object-surgery-receipt-count:
              (.get presentation object-surgery-receipt-count)
              conflict-surgery-receipt-count:
              (.get presentation conflict-surgery-receipt-count)
              rust-handler-manufactured:
              (.get presentation rust-handler-manufactured)
              replayable: (.get presentation replayable))))
    (.o kind: marlin-policy-projection-kind
        owner: (.get presentation policy-composition-owner)
        pack-kind: (.get policy-pack kind)
        pack-id: (.get policy-pack id)
        pack-owner: (.get policy-pack owner)
        pack-runtime-owner: (.get policy-pack runtime-owner)
        projection-chain-kind:
        (.get presentation projection-chain-kind)
        policy-facade-projection-chain-kind:
        (.get presentation policy-facade-projection-chain-kind)
        module-evaluation-receipt-kind:
        (.get presentation module-evaluation-receipt-kind)
        policy-projection-receipt-kind: marlin-policy-projection-kind
        native-projection-payload-kind: native-payload-kind
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

;;; Boundary: POO Flow compiles policy profiles into Rust-owned loop IR.
;; MarlinResult <- MarlinInput
(def (marlinPooLoopProgramCompilerReceipt profile-id resolved-policy-pack loop-program)
  (.o kind: marlin-poo-loop-program-compiler-receipt-kind
      profile-id: profile-id
      compiler-owner: "gerbil-poo-flow"
      resolved-policy-pack: resolved-policy-pack
      loop-program: loop-program
      scheme-boundary: "scheme-types-to-rust-types"
      serialization-boundary: "rust-owned-cli-trace-cross-process"))

;;; Boundary: Minimal real repair profile compiled as Scheme types for Rust.
;; MarlinResult <- MarlinInput
(def (marlin-real-repair-001-policy-digest)
  (make-vector 32 7))

;;; Boundary: Loop transition values use Rust-owned IR field names.
;; MarlinResult <- MarlinInput
(def (marlin-real-repair-001-transition transition-id-value
                                       from-value
                                       event-value
                                       action-value
                                       to-value)
  (.o transition_id: transition-id-value
      from: from-value
      event: event-value
      action: action-value
      to: to-value))

;;; Boundary: First real policy pack projection for the vertical loop mainline.
;; MarlinResult <- MarlinInput
(def (marlinRealRepair001ResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 42
      policy_digest: (marlin-real-repair-001-policy-digest)
      hot:
      (.o capability_mask: 5
          human_gate_mask: 1
          budget_caps:
          (.o max_attempts: 3
              max_cost_units: 1000
              max_wall_time_ms: 30000)
          graph_nodes:
          (vector
           (.o node_id: 1
               executor_id: 2
               capability_mask: 5
               resource_class_id: 4))
          graph_edges:
          (vector
           (.o from: 1
               to: 2))
          route_index:
          (.o buckets:
              (vector
               (.o bucket_id: 1
                   scope_mask: 255
                   target_id: 3)))
          resource_classes:
          (vector
           (.o resource_class_id: 4
               exclusive: #t))
          continuation_table:
          (vector
           (.o op: "stop_completed"))
          maker_profiles: (vector 11)
          checker_profiles: (vector 12))
      audit:
      (.o provenance:
          (vector
           (.o slot_id: 9
               winner_role: "planner"
               source_role_order: (vector "planner" "reviewer")
               merge: "union"))
          linearization: (vector "planner" "reviewer")
          diagnostics:
          (vector
           (.o code: "real-repair-001-policy-pack-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 1
               path: "gerbil/src/config-interface/modules/policy-pack.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector "real-repair-001 lowers POO profile into Rust loop IR")
          forced_slots:
          (vector
           (.o slot_id: 9
               hotness: "hot"))
          merge_receipts:
          (vector
           (.o slot_id: 9
               merge: "union"
               status: "applied")))))

;;; Boundary: First real LoopProgram emitted by the Scheme compiler surface.
;; MarlinResult <- MarlinInput
(def (marlinRealRepair001LoopProgram)
  (.o schema_version: 1
      program_id: "real-repair-001-scripted-loop"
      policy_epoch: 42
      policy_digest: (marlin-real-repair-001-policy-digest)
      mechanism_policies:
      (vector "reactive-tool-loop-base"
              "dynamic-graph-rewrite"
              "verification-gate")
      initial_state: "start"
      transitions:
      (vector
       (marlin-real-repair-001-transition
        "start-model"
        "start"
        "start"
        "invoke_model"
        "await-model")
       (marlin-real-repair-001-transition
        "model-tools"
        "await-model"
        "tool_request"
        "dispatch_tools"
        "await-tools")
       (marlin-real-repair-001-transition
        "tools-continue"
        "await-tools"
        "tool_receipt"
        "continue"
        "await-model")
       (marlin-real-repair-001-transition
        "dynamic-rewrite"
        "await-model"
        "model_event"
        "rewrite_graph"
        "rewritten")
       (marlin-real-repair-001-transition
        "verify-rewrite"
        "rewritten"
        "runtime_receipt"
        "verify"
        "verifying")
       (marlin-real-repair-001-transition
        "verification-stop"
        "verifying"
        "verification_receipt"
        "stop"
        "stopped"))))

;;; Boundary: Public compiler entrypoint for the first vertical loop case.
;; MarlinResult <- MarlinInput
(def (marlinRealRepair001LoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "real-repair-001/reactive-tool-loop"
   (marlinRealRepair001ResolvedPolicyPack)
   (marlinRealRepair001LoopProgram)))

;;; Boundary: Policy combination profile exercises memory, maker, rewrite, tool, checker.
;; MarlinResult <- MarlinInput
(def (marlin-policy-combination-matrix-policy-digest)
  (make-vector 32 15))

;;; Boundary: Combination transitions stay in Rust-owned LoopProgram IR names.
;; MarlinResult <- MarlinInput
(def (marlin-policy-combination-matrix-transition transition-id-value
                                                   from-value
                                                   event-value
                                                   action-value
                                                   to-value)
  (.o transition_id: transition-id-value
      from: from-value
      event: event-value
      action: action-value
      to: to-value))

;;; Boundary: Real policy combination pack preserves hot and audit evidence.
;; MarlinResult <- MarlinInput
(def (marlinPolicyCombinationMatrixResolvedPolicyPack)
  (.o schema_version: 1
      policy_epoch: 15
      policy_digest: (marlin-policy-combination-matrix-policy-digest)
      hot:
      (.o capability_mask: 7
          human_gate_mask: 1
          budget_caps:
          (.o max_attempts: 3
              max_cost_units: 1000
              max_wall_time_ms: 30000)
          graph_nodes:
          (vector
           (.o node_id: 1
               executor_id: 21
               capability_mask: 1
               resource_class_id: 4)
           (.o node_id: 2
               executor_id: 22
               capability_mask: 2
               resource_class_id: 4)
           (.o node_id: 3
               executor_id: 23
               capability_mask: 4
               resource_class_id: 4))
          graph_edges:
          (vector
           (.o from: 1
               to: 2)
           (.o from: 2
               to: 3))
          route_index:
          (.o buckets:
              (vector
               (.o bucket_id: 1
                   scope_mask: 255
                   target_id: 3)))
          resource_classes:
          (vector
           (.o resource_class_id: 4
               exclusive: #t))
          continuation_table:
          (vector
           (.o op: "memory_rewrite_checker_stop"))
          maker_profiles: (vector 21)
          checker_profiles: (vector 22))
      audit:
      (.o provenance:
          (vector
           (.o slot_id: 31
               winner_role: "memory"
               source_role_order: (vector "memory" "maker" "checker")
               merge: "ordered_append")
           (.o slot_id: 32
               winner_role: "maker"
               source_role_order: (vector "memory" "maker" "checker")
               merge: "ordered_append")
           (.o slot_id: 33
               winner_role: "checker"
               source_role_order: (vector "memory" "maker" "checker")
               merge: "ordered_append"))
          linearization: (vector "memory" "maker" "rewrite" "tool" "checker")
          diagnostics:
          (vector
           (.o code: "policy-combination-matrix-ok"
               severity: "info"))
          source_locations:
          (vector
           (.o source_location_id: 2
               path: "gerbil/src/config-interface/modules/policy-pack.ss"
               line: 1
               column: 1))
          explanation_strings:
          (vector "policy combination matrix lowers memory, maker, rewrite, tool, checker into Rust loop IR")
          forced_slots:
          (vector
           (.o slot_id: 31
               hotness: "hot")
           (.o slot_id: 32
               hotness: "hot")
           (.o slot_id: 33
               hotness: "hot"))
          merge_receipts:
          (vector
           (.o slot_id: 31
               merge: "ordered_append"
               status: "applied")
           (.o slot_id: 32
               merge: "ordered_append"
               status: "applied")
           (.o slot_id: 33
               merge: "ordered_append"
               status: "applied")))))

;;; Boundary: Policy combination LoopProgram is emitted by the Scheme compiler surface.
;; MarlinResult <- MarlinInput
(def (marlinPolicyCombinationMatrixLoopProgram)
  (.o schema_version: 1
      program_id: "policy-combination-memory-rewrite-checker"
      policy_epoch: 15
      policy_digest: (marlin-policy-combination-matrix-policy-digest)
      mechanism_policies:
      (vector "real-policy-003-maker-checker"
              "real-policy-004-dynamic-rewrite"
              "real-policy-005-memory-recall")
      initial_state: "start"
      transitions:
      (vector
       (marlin-policy-combination-matrix-transition
        "start-memory"
        "start"
        "start"
        "read_memory"
        "memory-ready")
       (marlin-policy-combination-matrix-transition
        "memory-maker"
        "memory-ready"
        "runtime_receipt"
        "invoke_model"
        "await-maker")
       (marlin-policy-combination-matrix-transition
        "maker-rewrite"
        "await-maker"
        "model_event"
        "rewrite_graph"
        "rewritten")
       (marlin-policy-combination-matrix-transition
        "rewrite-tool"
        "rewritten"
        "runtime_receipt"
        "dispatch_tools"
        "await-tool")
       (marlin-policy-combination-matrix-transition
        "tool-checker"
        "await-tool"
        "tool_receipt"
        "verify"
        "await-checker")
       (marlin-policy-combination-matrix-transition
        "checker-stop"
        "await-checker"
        "verification_receipt"
        "stop"
        "stopped"))))

;;; Boundary: Public compiler entrypoint for the first policy combination case.
;; MarlinResult <- MarlinInput
(def (marlinPolicyCombinationMatrixLoopProgramCompilerReceipt)
  (marlinPooLoopProgramCompilerReceipt
   "policy-combination/memory-rewrite-checker"
   (marlinPolicyCombinationMatrixResolvedPolicyPack)
   (marlinPolicyCombinationMatrixLoopProgram)))

;;; Boundary: Module evaluation receipt summarizes Scheme-owned composition.
;; MarlinResult <- MarlinInput
(def (marlinPolicyModuleEvaluationReceipt policy-projection)
  (.o kind: (.get policy-projection module-evaluation-receipt-kind)
      pack-id: (.get policy-projection pack-id)
      owner: (.get policy-projection import-graph-owner)
      import-graph-owner: (.get policy-projection import-graph-owner)
      option-merge-owner: (.get policy-projection option-merge-owner)
      extension-composition-owner:
      (.get policy-projection extension-composition-owner)
      policy-composition-owner:
      (.get policy-projection policy-composition-owner)
      replayable: (.get policy-projection replayable)))

;;; Boundary: Budget receipt is Rust-owned validation metadata.
;; MarlinResult <- MarlinInput
(def (marlinPolicyBudgetReceipt policy-projection)
  (.o kind: marlin-policy-budget-receipt-kind
      pack-id: (.get policy-projection pack-id)
      owner: (.get policy-projection budget-receipt-owner)
      budget-owner: (.get policy-projection budget-receipt-owner)
      runtime-lifecycle-owner:
      (.get policy-projection runtime-lifecycle-owner)
      policy-composition-owner:
      (.get policy-projection policy-composition-owner)
      replayable: (.get policy-projection replayable)))

;;; Boundary: Catalog receipt names Rust handler lookup without creating one.
;; MarlinResult <- MarlinInput
(def (marlinPolicyCatalogResolutionReceipt policy-projection)
  (let (native-payload
        (.get policy-projection native-projection-payload))
    (.o kind: marlin-policy-catalog-resolution-receipt-kind
        pack-id: (.get policy-projection pack-id)
        owner: (.get policy-projection catalog-resolution-receipt-owner)
        catalog-handler-lookup-owner:
        (.get policy-projection catalog-resolution-receipt-owner)
        allowed-hook-ids: (.get native-payload allowed-hook-ids)
        allowed-hook-count: (.get native-payload allowed-hook-count)
        rust-handler-manufactured:
        (.get policy-projection rust-handler-manufactured)
        scheme-manufactures-rust-handlers: #f
        replayable: (.get policy-projection replayable))))

;;; Boundary: Fixed five-family chain for prefab and custom policy packs.
;; MarlinResult <- MarlinInput
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
    (.o kind: marlin-policy-projection-chain-receipt-kind
        pack-id: (.get policy-pack id)
        receipt-family-count: 5
        receipt-family-ids: receipt-family-id-values
        module-evaluation-receipt: module-evaluation-receipt-value
        policy-projection-receipt: policy-projection-value
        native-payload: native-payload-value
        native-projection-payload: native-payload-value
        budget-receipt: budget-receipt-value
        catalog-resolution-receipt: catalog-resolution-receipt-value
        module-evaluation-receipt-owner:
        (.get module-evaluation-receipt-value owner)
        policy-projection-receipt-owner:
        (.get policy-projection-value owner)
        native-projection-payload-owner:
        (.get policy-projection-value native-projection-payload-owner)
        budget-receipt-owner:
        (.get budget-receipt-value owner)
        catalog-resolution-receipt-owner:
        (.get catalog-resolution-receipt-value owner)
        catalog-resolution-allowed-hook-count:
        catalog-resolution-allowed-hook-count-value
        replayable: #t)))

;;; Boundary: Public receipt helper keeps a stable varargs API.
;; MarlinResult <- MarlinInput
(def (marlinPolicyProjectionReceipts policy-pack . maybe-native-payload)
  (if (pair? maybe-native-payload)
    (marlin-policy-projection-receipts/direct
     policy-pack
     (car maybe-native-payload))
    (marlin-policy-projection-receipts/direct policy-pack)))

;;; Boundary: Fixed receipt chain for module -> policy -> Rust validation.
;; MarlinResult <- MarlinInput
(def (marlinPolicyProjectionChainReceipt policy-pack . maybe-native-payload)
  (if (pair? maybe-native-payload)
    (marlin-policy-projection-receipts/direct
     policy-pack
     (car maybe-native-payload))
    (marlin-policy-projection-receipts/direct policy-pack)))
