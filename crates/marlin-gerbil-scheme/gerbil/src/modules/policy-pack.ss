;;; -*- Gerbil -*-
;;; Boundary: Prefab policy packs and pack projection receipts.

package: modules

(import (only-in :clan/poo/object .get .o)
        :modules/kinds
        :modules/core
        :modules/policy-object
        :modules/evaluation)

(export marlinPolicyPack
        defmarlin-policy-pack
        marlinDefaultPolicyPack
        marlinPackCatalog
        marlin-pack-catalog-find
        marlin-pack-catalog-root
        marlinPolicyPackInventory
        marlinPolicyPackPresentation
        marlinPolicyProjection)

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
(def (marlin-default-policy-pack-object
      family-value
      object-id-value
      object-value)
  (marlinPolicyObject
   family-value
   object-id-value
   object-value
   '((owner . "marlin") (surface . "default-prefab-object"))))

;;; Boundary: The furnished default pack starts from coherent policy families.
;; MarlinResult <- MarlinInput
(def (marlin-default-policy-pack-objects)
  (list
   (marlin-default-policy-pack-object
    "workspace-policy"
    "default-workspace"
    (.o isolation: "shared-worktree"
        branch-mode: "policy-branch"
        snapshot-mode: "typed-receipt-snapshot"))
   (marlin-default-policy-pack-object
    "session-policy"
    "default-session"
    (.o sharing: "shared"
        isolation: "branch-isolated"
        snapshot: "enabled"))
   (marlin-default-policy-pack-object
    "agent-policy"
    "default-agent"
    (.o root-agent: "root-agent"
        subagent-policy: "module-managed"))
   (marlin-default-policy-pack-object
    "hook-selection-policy"
    "default-hook"
    (.o hook-id: "runtime-catalog-default-hook"
        action: "register"))
   (marlin-default-policy-pack-object
    "model-route-policy"
    "default-model-route"
    (.o provider: "openai"
        model: "gpt-5.4"
        route: "interactive"))
   (marlin-default-policy-pack-object
    "continuation-profile-policy"
    "default-continuation"
    (.o profile: "balanced"
        graph-intent: "loop-graph"))
   (marlin-default-policy-pack-object
    "human-review-policy"
    "default-human-review"
    (.o trigger: "high-risk-tool"
        reviewer: "root-agent"))
   (marlin-default-policy-pack-object
    "failure-recovery-policy"
    "default-failure-recovery"
    (.o retry-budget: "bounded"
        recovery: "receipt-driven"))
   (marlin-default-policy-pack-object
    "catalog-projection-policy"
    "default-catalog-projection"
    (.o projection-target: "rust-catalog-handlers"
        resolution-owner: "rust"))))

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
          "marlinPolicyProjection")
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
