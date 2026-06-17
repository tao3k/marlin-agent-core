;;; -*- Gerbil -*-
;;; Boundary: Module owns the public POO module interface for downstream users.

package: marlin/modules

(import (only-in :clan/poo/object .all-slots .get .has? .o .ref object?)
        (only-in :clan/poo/type String)
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option)

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
        marlin-policy-pack-presentation-kind
        marlin-policy-object-kind
        marlin-policy-object-operation-kind
        marlin-policy-object-surgery-receipt-kind
        marlin-module-import-kind
        marlin-import-source-ref-kind
        marlin-import-local-source-kind
        marlinModules
        marlinPolicyExtension
        defmarlin-policy-extension
        marlinPolicyModule
        defmarlin-policy-module
        marlinPolicyObject
        marlin-policy-object?
        marlin-policy-object-id
        marlin-policy-object-family
        marlin-add-object
        marlin-remove-object
        marlin-disable-object
        marlin-replace-object
        marlin-policy-object-operation?
        marlin-policy-pack-apply-operations
        marlinPolicyPack
        defmarlin-policy-pack
        marlinPackCatalog
        marlin-pack-catalog-find
        marlin-pack-catalog-root
        marlinPolicyPackPresentation
        marlinModuleCatalog
        marlin-import
        marlin-imports
        marlin-imports-append
        marlin-extensions
        marlin-extensions-append
        marlin-import?
        marlin-policy-extension?
        marlin-policy-module?
        marlin-module-import-source-ref?
        marlin-module-import-local-source?
        marlin-module-import-normalize-source
        marlin-source-ref
        marlin-local-source
        marlin-module-interface
        marlin-string-required
        marlin-string-constant
        marlin-string-default
        marlin-string-optional
        marlin-module-runtime-import
        marlin-module-option-configs
        marlin-module-option-schemas
        marlin-module-find-schema
        marlin-module-missing-schema-receipt
        marlin-module-option-validation-receipts
        marlin-module-validation-receipts
        marlin-module-apply
        marlin-module-evaluate
        marlin-module-workflow
        marlin-module-catalog-find
        marlin-module-catalog-root
        marlinEvalModules
        marlinModuleSystemPresentation
        marlin-policy-extension-object-count
        marlin-policy-module-substrate-gate
        marlin-policy-module-workflow)

;;; Boundary: Public user module kind is stable across module-lib helpers.
;; MarlinResult <- MarlinInput
(def marlin-modules-kind
  "marlin.modules.v1")

;;; Boundary: Workflow helpers keep runtime projections out of user config files.
;; MarlinResult <- MarlinInput
(def marlin-module-workflow-kind
  "marlin.modules.workflow.v1")

;;; Boundary: Catalogs are Scheme values, not path/evaluator conventions.
;; MarlinResult <- MarlinInput
(def marlin-module-catalog-kind
  "marlin.modules.catalog.v1")

;;; Boundary: evalModules returns a receipt bundle, not raw runtime config.
;; MarlinResult <- MarlinInput
(def marlin-eval-modules-result-kind
  "marlin.modules.eval-result.v1")

;;; Boundary: Presentations are scalar receipts for the whole user module surface.
;; MarlinResult <- MarlinInput
(def marlin-module-system-presentation-kind
  "marlin.modules.system-presentation.v1")

;;; Boundary: Projection chains name stable Rust-owned receipt handoff points.
;; MarlinResult <- MarlinInput
(def marlin-module-projection-chain-kind
  "marlin.modules.projection-chain.v1")

;;; Boundary: User .ss files export POO extension objects managed by modules.
;; MarlinResult <- MarlinInput
(def marlin-policy-extension-kind
  "marlin.modules.policy-extension-object.v1")

;;; Boundary: Policy modules are Scheme-owned POO modules, not Rust DSLs.
;; MarlinResult <- MarlinInput
(def marlin-policy-module-kind
  "marlin.modules.policy-module.v1")

;;; Boundary: Policy workflows add policy substrate metadata to module workflow.
;; MarlinResult <- MarlinInput
(def marlin-policy-module-workflow-kind
  "marlin.modules.policy-workflow.v1")

;;; Boundary: The substrate gate is a stable receipt for Rust validation.
;; MarlinResult <- MarlinInput
(def marlin-policy-substrate-gate-kind
  "marlin.modules.policy-substrate-gate.v1")

;;; Boundary: Policy packs are Scheme/POO prefab bundles, not Rust DSLs.
;; MarlinResult <- MarlinInput
(def marlin-policy-pack-kind
  "marlin.modules.policy-pack.v1")

;;; Boundary: Pack catalogs collect prefab bundles without parsing .ss text.
;; MarlinResult <- MarlinInput
(def marlin-pack-catalog-kind
  "marlin.modules.policy-pack-catalog.v1")

;;; Boundary: Pack presentations are scalar receipts for Rust/debug tooling.
;; MarlinResult <- MarlinInput
(def marlin-policy-pack-presentation-kind
  "marlin.modules.policy-pack-presentation.v1")

;;; Boundary: Policy objects are the POO "furniture" inside prefab packs.
;; MarlinResult <- MarlinInput
(def marlin-policy-object-kind
  "marlin.modules.policy-object.v1")

;;; Boundary: Object surgery keeps add/remove/disable/replace in Scheme.
;; MarlinResult <- MarlinInput
(def marlin-policy-object-operation-kind
  "marlin.modules.policy-object-operation.v1")

;;; Boundary: Surgery receipts prove Scheme changed objects, not Rust handlers.
;; MarlinResult <- MarlinInput
(def marlin-policy-object-surgery-receipt-kind
  "marlin.modules.policy-object-surgery-receipt.v1")

;;; Boundary: Public import specs name user files and exported profiles.
;; MarlinResult <- MarlinInput
(def marlin-module-import-kind
  "marlin.modules.import.v1")

;;; Boundary: Import source references keep source metadata as POO objects.
;; MarlinResult <- MarlinInput
(def marlin-import-source-ref-kind
  "marlin.modules.import.source-ref.v1")

;;; Boundary: Local sources name user workspace files without parsing them.
;; MarlinResult <- MarlinInput
(def marlin-import-local-source-kind
  "marlin.modules.import.local-source.v1")

;;; Boundary: Internal prototype anchors module defaults for imported interfaces.
;; MarlinResult <- MarlinInput
(def marlin-module-prototype
  (.o kind: marlin-modules-kind
      id: "anonymous-marlin-module"
      imports: '()
      extensions: '()
      scripts: '()
      options: (.o)
      schemas: (.o)
      metadata: '()))

;;; Boundary: Interface objects carry schemas outside user config records.
;; MarlinResult <- MarlinInput
(def (marlin-module-interface interface-id-value schema-object metadata-value)
  (.o (:: @ (list marlin-module-prototype))
      id: interface-id-value
      schemas: schema-object
      metadata: metadata-value))

;;; Boundary: String option helpers keep user interface modules concise.
;; MarlinResult <- MarlinInput
(def (marlin-string-required)
  (.o type: String))

;;; Boundary: String constant helpers model fixed interface values.
;; MarlinResult <- MarlinInput
(def (marlin-string-constant constant-value)
  (.o type: String
      constant: constant-value))

;;; Boundary: String default helpers model optional defaults.
;; MarlinResult <- MarlinInput
(def (marlin-string-default default-value)
  (.o type: String
      default: default-value))

;;; Boundary: String optional helpers model optional schema slots.
;; MarlinResult <- MarlinInput
(def (marlin-string-optional)
  (.o type: String
      optional?: #t))

;;; Boundary: Config object lookup supports a record-like user interface.
;; MarlinResult <- MarlinInput
(def (marlin-module-object-has-slot? object slot-name)
  (member slot-name (.all-slots object)))

;;; Boundary: Missing config fields fall back to the module interface defaults.
;; MarlinResult <- MarlinInput
(def (marlin-module-object-ref/default object slot-name default-value)
  (if (marlin-module-object-has-slot? object slot-name)
    (.ref object slot-name)
    default-value))

;;; Boundary: Public user API mirrors typed config records from module systems.
;; MarlinResult <- MarlinInput
(def (marlinModules interface module-config)
  (let ((config-values
         (marlin-module-object-ref/default
          module-config
          'config
          (.o))))
    (.o (:: @ (list interface))
        id:
        (marlin-module-object-ref/default module-config 'id (.get interface id))
        imports:
        (marlin-module-object-ref/default module-config 'imports '())
        extensions:
        (marlin-module-object-ref/default module-config 'extensions '())
        scripts:
        (marlin-module-object-ref/default module-config 'scripts '())
        options: config-values
        metadata:
        (marlin-module-object-ref/default
         module-config
         'metadata
         (.get interface metadata)))))

;;; Boundary: Policy extensions are POO objects authored by .ss files.
;; MarlinResult <- MarlinInput
(def (marlinPolicyExtension extension-value source-value . maybe-metadata-value)
  (let (metadata-value
        (if (null? maybe-metadata-value)
          '()
          (car maybe-metadata-value)))
    (.o (:: @ (list extension-value))
        policy-extension-kind: marlin-policy-extension-kind
        policy-extension-object: #t
        policy-extension-source: source-value
        policy-extension-managed-by: "gerbil-module-system"
        policy-extension-projection-owner: "gerbil-poo"
        policy-extension-runtime-owner: "rust"
        policy-extension-metadata: metadata-value)))

;;; Boundary: Level-1 user API names exported POO extension objects directly.
;; MarlinResult <- MarlinInput
(defrules defmarlin-policy-extension ()
  ((_ binding
      (source source-value)
      (object extension-object)
      (metadata metadata-value))
   (def binding
     (marlinPolicyExtension
      extension-object
      source-value
      metadata-value)))
  ((_ binding
      (source source-value)
      (object extension-object))
   (def binding
     (marlinPolicyExtension
      extension-object
      source-value
      '()))))

;;; Boundary: Predicate identifies module-managed policy extension objects.
;; MarlinResult <- MarlinInput
(def (marlin-policy-extension? value)
  (and (object? value)
       (marlin-module-object-has-slot? value 'policy-extension-kind)
       (string=? (.get value policy-extension-kind)
                 marlin-policy-extension-kind)))

;;; Boundary: Receipts count extension objects without inspecting policy internals.
;; MarlinResult <- MarlinInput
(def (marlin-policy-extension-object-count extension-values)
  (let loop ((remaining extension-values)
             (count 0))
    (if (null? remaining)
      count
      (loop (cdr remaining)
            (if (marlin-policy-extension? (car remaining))
              (+ count 1)
              count)))))

;;; Boundary: Policy modules keep policy composition in Scheme/POO.
;; MarlinResult <- MarlinInput
(def (marlinPolicyModule interface module-config)
  (let (module-value (marlinModules interface module-config))
    (.o (:: @ (list module-value))
        kind: marlin-policy-module-kind
        module-kind: marlin-modules-kind
        id: (.get module-value id)
        policy-family:
        (marlin-module-object-ref/default
         module-config
         'policy-family
         "extension-policy")
        projection-target:
        (marlin-module-object-ref/default
         module-config
         'projection-target
         "extension-policy-receipt")
        receipt-kind:
        (marlin-module-object-ref/default
         module-config
         'receipt-kind
         "marlin-deck-runtime.extension-receipt.v1")
        gate-profile:
        (marlin-module-object-ref/default
         module-config
         'gate-profile
         "policy-substrate")
        rust-kernel-owner: "rust"
        scheme-policy-owner: "gerbil-poo"
        replayable: #t)))

;;; Boundary: Level-1 user API expands to the POO policy module object.
;; MarlinResult <- MarlinInput
(defrules defmarlin-policy-module ()
  ((_ binding
      interface
      (id module-id)
      (imports import-value ...)
      (config config-object)
      (extensions extension-value ...)
      (scripts script-value ...)
      (policy-family policy-family-value)
      (projection-target projection-target-value)
      (receipt-kind receipt-kind-value)
      (gate-profile gate-profile-value)
      (metadata metadata-value))
   (def binding
     (marlinPolicyModule
      interface
      (.o id: module-id
          imports: (marlin-imports import-value ...)
          config: config-object
          extensions: (marlin-extensions extension-value ...)
          scripts: (list script-value ...)
          policy-family: policy-family-value
          projection-target: projection-target-value
          receipt-kind: receipt-kind-value
          gate-profile: gate-profile-value
          metadata: metadata-value)))))

;;; Boundary: Policy objects wrap existing POO values without changing handlers.
;; MarlinResult <- MarlinInput
(def (marlinPolicyObject
      family-value
      object-id-value
      object-value
      . maybe-metadata-value)
  (let (metadata-value
        (if (null? maybe-metadata-value)
          '()
          (car maybe-metadata-value)))
    (.o (:: @ (list object-value))
        id: object-id-value
        policy-object-kind: marlin-policy-object-kind
        policy-object-family: family-value
        policy-object-id: object-id-value
        policy-object-disabled: #f
        policy-object-payload: object-value
        policy-object-metadata: metadata-value
        policy-object-owner: "gerbil-poo"
        rust-handler-manufactured: #f)))

;;; Boundary: Predicate identifies managed policy objects by sidecar kind.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object? value)
  (and (object? value)
       (.has? value policy-object-kind)
       (string=? (.get value policy-object-kind)
                 marlin-policy-object-kind)))

;;; Boundary: Object ids are stable within a policy object family.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-id value)
  (.get value policy-object-id))

;;; Boundary: Families group policy objects without Rust knowing merge rules.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-family value)
  (.get value policy-object-family))

;;; Boundary: Disabled objects remain visible in presentations and receipts.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-disable object-value reason-value)
  (let (payload-value
        (.get object-value policy-object-payload))
    (.o (:: @ (list payload-value))
        id: (marlin-policy-object-id object-value)
        policy-object-kind: marlin-policy-object-kind
        policy-object-family: (marlin-policy-object-family object-value)
        policy-object-id: (marlin-policy-object-id object-value)
        policy-object-disabled: #t
        policy-object-disabled-reason: reason-value
        policy-object-payload: payload-value
        policy-object-metadata: (.get object-value policy-object-metadata)
        policy-object-owner: "gerbil-poo"
        rust-handler-manufactured: #f)))

;;; Boundary: Object operations carry intent plus receipts, not handler code.
;; MarlinResult <- MarlinInput
(def (marlinPolicyObjectOperation
      operation-value
      family-value
      target-id-value
      object-value
      replacement-value
      maybe-values)
  (let ((reason-value
         (if (null? maybe-values)
           "policy-object-surgery"
           (car maybe-values)))
        (metadata-value
         (if (or (null? maybe-values)
                 (null? (cdr maybe-values)))
           '()
           (cadr maybe-values))))
    (.o kind: marlin-policy-object-operation-kind
        operation: operation-value
        family: family-value
        target-id: target-id-value
        operation-object: object-value
        operation-replacement: replacement-value
        reason: reason-value
        metadata: metadata-value
        owner: "gerbil-poo"
        rust-handler-manufactured: #f)))

;;; Boundary: Add operations append a managed policy object to the prefab pack.
;; MarlinResult <- MarlinInput
(def (marlin-add-object object-value . maybe-values)
  (marlinPolicyObjectOperation
   "add"
   (marlin-policy-object-family object-value)
   (marlin-policy-object-id object-value)
   object-value
   #f
   maybe-values))

;;; Boundary: Remove operations delete matching furniture by family/id.
;; MarlinResult <- MarlinInput
(def (marlin-remove-object family-value target-id-value . maybe-values)
  (marlinPolicyObjectOperation
   "remove"
   family-value
   target-id-value
   #f
   #f
   maybe-values))

;;; Boundary: Disable operations keep the object visible but inactive.
;; MarlinResult <- MarlinInput
(def (marlin-disable-object family-value target-id-value . maybe-values)
  (marlinPolicyObjectOperation
   "disable"
   family-value
   target-id-value
   #f
   #f
   maybe-values))

;;; Boundary: Replace operations swap one family/id object for another POO object.
;; MarlinResult <- MarlinInput
(def (marlin-replace-object
      family-value
      target-id-value
      replacement-value
      . maybe-values)
  (marlinPolicyObjectOperation
   "replace"
   family-value
   target-id-value
   #f
   replacement-value
   maybe-values))

;;; Boundary: Operation detection is typed by kind, not list syntax.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-operation? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-policy-object-operation-kind)))

;;; Boundary: Object surgery matches only managed family/id pairs.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-matches? object-value family-value target-id-value)
  (and (marlin-policy-object? object-value)
       (equal? (marlin-policy-object-family object-value) family-value)
       (equal? (marlin-policy-object-id object-value) target-id-value)))

;;; Boundary: Receipt ids stay scalar for Rust/debug projections.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-id/default object-value default-value)
  (if (marlin-policy-object? object-value)
    (marlin-policy-object-id object-value)
    default-value))

;;; Boundary: Surgery receipts prove Scheme object composition decisions.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-surgery-receipt operation-value matched-count-value)
  (.o kind: marlin-policy-object-surgery-receipt-kind
      operation: (.get operation-value operation)
      family: (.get operation-value family)
      target-id: (.get operation-value target-id)
      object-id:
      (marlin-policy-object-id/default
       (.get operation-value operation-object)
       #f)
      replacement-id:
      (marlin-policy-object-id/default
       (.get operation-value operation-replacement)
       #f)
      matched?: (> matched-count-value 0)
      matched-count: matched-count-value
      owner: "gerbil-poo"
      rust-handler-manufactured: #f))

;;; Boundary: A single operation transforms objects and returns one receipt.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-apply-operation object-values operation-value)
  (let* ((operation-name (.get operation-value operation))
         (family-value (.get operation-value family))
         (target-id-value (.get operation-value target-id)))
    (cond
     ((string=? operation-name "add")
      (.o policy-objects:
          (append object-values
                  (list (.get operation-value operation-object)))
          matched-count: 1))
     ((string=? operation-name "remove")
      (let ((matched
             (filter (lambda (candidate)
                       (marlin-policy-object-matches?
                        candidate
                        family-value
                        target-id-value))
                     object-values))
            (kept
             (filter (lambda (candidate)
                       (not
                        (marlin-policy-object-matches?
                         candidate
                         family-value
                         target-id-value)))
                     object-values)))
        (.o policy-objects: kept
            matched-count: (length matched))))
     ((string=? operation-name "disable")
      (let (matched
            (filter (lambda (candidate)
                      (marlin-policy-object-matches?
                       candidate
                       family-value
                       target-id-value))
                    object-values))
        (.o policy-objects:
            (map (lambda (candidate)
                   (if (marlin-policy-object-matches?
                        candidate
                        family-value
                        target-id-value)
                     (marlin-policy-object-disable
                      candidate
                      (.get operation-value reason))
                     candidate))
                 object-values)
            matched-count: (length matched))))
     ((string=? operation-name "replace")
      (let (matched
            (filter (lambda (candidate)
                      (marlin-policy-object-matches?
                       candidate
                       family-value
                       target-id-value))
                    object-values))
        (.o policy-objects:
            (map (lambda (candidate)
                   (if (marlin-policy-object-matches?
                        candidate
                        family-value
                        target-id-value)
                     (.get operation-value operation-replacement)
                     candidate))
                 object-values)
            matched-count: (length matched))))
     (else
      (error "unknown marlin policy object operation" operation-name)))))

;;; Boundary: Pack object surgery is deterministic and receipt-producing.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-apply-operations object-values operation-values)
  (let loop ((remaining operation-values)
             (current object-values)
             (receipts '())
             (add-count 0)
             (remove-count 0)
             (disable-count 0)
             (replace-count 0)
             (matched-receipt-count 0))
    (if (null? remaining)
      (.o policy-objects: current
          surgery-receipts: (reverse receipts)
          add-operation-count: add-count
          remove-operation-count: remove-count
          disable-operation-count: disable-count
          replace-operation-count: replace-count
          matched-surgery-receipt-count: matched-receipt-count)
      (let* ((operation-value (car remaining))
             (operation-name (.get operation-value operation))
             (operation-result
              (marlin-policy-pack-apply-operation current operation-value))
             (receipt
              (marlin-policy-object-surgery-receipt
               operation-value
               (.get operation-result matched-count))))
        (loop (cdr remaining)
              (.get operation-result policy-objects)
              (cons receipt receipts)
              (+ add-count
                 (if (string=? operation-name "add") 1 0))
              (+ remove-count
                 (if (string=? operation-name "remove") 1 0))
              (+ disable-count
                 (if (string=? operation-name "disable") 1 0))
              (+ replace-count
                 (if (string=? operation-name "replace") 1 0))
              (+ matched-receipt-count
                 (if (> (.get operation-result matched-count) 0) 1 0)))))))

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
         (default-policy-objects
          (marlin-module-object-ref/default
           pack-config
           'policy-objects
           '()))
         (object-operations
          (marlin-module-object-ref/default
           pack-config
           'object-operations
           '()))
         (operation-result
          (marlin-policy-pack-apply-operations
           default-policy-objects
           object-operations)))
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
        default-policy-objects: default-policy-objects
        policy-objects: (.get operation-result policy-objects)
        object-operations: object-operations
        object-surgery-receipts: (.get operation-result surgery-receipts)
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

;;; Boundary: Disabled object counts keep object surgery auditable.
;; MarlinResult <- MarlinInput
(def (marlin-policy-disabled-object-count object-values)
  (length
   (filter (lambda (object-value)
             (and (marlin-policy-object? object-value)
                  (.get object-value policy-object-disabled)))
           object-values)))

;;; Boundary: Pack presentation is the stable projection pattern for Rust.
;; MarlinResult <- MarlinInput
(def (marlinPolicyPackPresentation policy-pack)
  (let* ((module-system-presentation
          (marlinModuleSystemPresentation
           (.get policy-pack catalog)
           (.get policy-pack root-module-id)
           (.get policy-pack allowed-hook-ids))))
    (.o kind: marlin-policy-pack-presentation-kind
        pack-kind: (.get policy-pack kind)
        pack-id: (.get policy-pack id)
        pack-owner: (.get policy-pack owner)
        pack-runtime-owner: (.get policy-pack runtime-owner)
        pack-catalog-kind: marlin-pack-catalog-kind
        module-system-presentation-kind:
        (.get module-system-presentation kind)
        module-system-projection-chain-kind:
        (.get module-system-presentation projection-chain-kind)
        root-module-id: (.get policy-pack root-module-id)
        root-module-kind: (.get module-system-presentation root-module-kind)
        policy-object-count: (.get policy-pack policy-object-count)
        disabled-policy-object-count:
        (.get policy-pack disabled-policy-object-count)
        object-operation-count: (.get policy-pack object-operation-count)
        object-surgery-receipt-count:
        (.get policy-pack object-surgery-receipt-count)
        add-operation-count: (.get policy-pack add-operation-count)
        remove-operation-count: (.get policy-pack remove-operation-count)
        disable-operation-count: (.get policy-pack disable-operation-count)
        replace-operation-count: (.get policy-pack replace-operation-count)
        matched-surgery-receipt-count:
        (.get policy-pack matched-surgery-receipt-count)
        allowed-hook-count: (length (.get policy-pack allowed-hook-ids))
        user-entrypoints:
        '("marlinPolicyPack"
          "defmarlin-policy-pack"
          "marlinPolicyObject"
          "marlin-add-object"
          "marlin-remove-object"
          "marlin-disable-object"
          "marlin-replace-object"
          "marlinPolicyPackPresentation")
        module-evaluation-receipt-kind:
        (.get module-system-presentation module-evaluation-receipt-kind)
        import-graph-owner:
        (.get module-system-presentation import-graph-owner)
        option-merge-owner:
        (.get module-system-presentation option-merge-owner)
        extension-composition-owner:
        (.get module-system-presentation extension-composition-owner)
        native-projection-payload-owner:
        (.get module-system-presentation native-projection-payload-owner)
        budget-receipt-owner:
        (.get module-system-presentation budget-receipt-owner)
        catalog-resolution-receipt-owner:
        (.get module-system-presentation catalog-resolution-receipt-owner)
        rust-parses-scheme-source:
        (.get policy-pack rust-parses-scheme-source)
        rust-handler-manufactured:
        (.get policy-pack rust-handler-manufactured)
        scheme-policy-owner: "gerbil-poo"
        rust-kernel-owner: "rust"
        replayable: #t)))

;;; Boundary: Public catalogs collect typed module values for evaluation.
;; MarlinResult <- MarlinInput
(def (marlinModuleCatalog . module-values)
  (.o kind: marlin-module-catalog-kind
      modules: module-values))

;;; Boundary: Local source objects mirror Jsonnet-style structured config.
;; MarlinResult <- MarlinInput
(def (marlin-local-source source-path)
  (.o kind: marlin-import-local-source-kind
      path: source-path))

;;; Boundary: Source refs wrap concrete source kinds for future package sources.
;; MarlinResult <- MarlinInput
(def (marlin-source-ref source-value)
  (.o kind: marlin-import-source-ref-kind
      source: source-value))

;;; Boundary: Source-ref detection keeps normalization typed by object kind.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-source-ref? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-import-source-ref-kind)))

;;; Boundary: Local-source detection keeps normalization typed by object kind.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-local-source? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-import-local-source-kind)))

;;; Boundary: User imports accept path strings or explicit source objects.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-normalize-source source-value)
  (cond
   ((string? source-value)
    (marlin-source-ref (marlin-local-source source-value)))
   ((marlin-module-import-source-ref? source-value)
    source-value)
   ((marlin-module-import-local-source? source-value)
    (marlin-source-ref source-value))
   (else source-value)))

;;; Boundary: Import specs keep user config close to POO extension objects.
;; MarlinResult <- MarlinInput
(def (make-marlin-import source-ref-value profile-value)
  (.o kind: marlin-module-import-kind
      source-ref: source-ref-value
      profile: profile-value))

;;; Boundary: Public import helper accepts profile or path/profile forms.
;; MarlinResult <- MarlinInput
(def (marlin-import . import-values)
  (cond
   ((= (length import-values) 1)
    (make-marlin-import #f (car import-values)))
   ((= (length import-values) 2)
    (make-marlin-import
     (marlin-module-import-normalize-source (car import-values))
     (cadr import-values)))
    (else
    (error "marlin-import expects profile or source/profile"))))

;;; Boundary: Import lists are explicit values consumed by POO slot methods.
;; MarlinResult <- MarlinInput
(def (marlin-imports . import-values)
  import-values)

;;; Boundary: POO slot methods lazily append child imports to inherited imports.
;; MarlinResult <- MarlinInput
(def (marlin-imports-append inherited-imports direct-imports)
  (append inherited-imports direct-imports))

;;; Boundary: Extension lists let agent-authored POO objects stay first-class.
;; MarlinResult <- MarlinInput
(def (marlin-extensions . extension-values)
  extension-values)

;;; Boundary: POO slot methods lazily append child extension objects.
;; MarlinResult <- MarlinInput
(def (marlin-extensions-append inherited-extensions direct-extensions)
  (append inherited-extensions direct-extensions))

;;; Boundary: Import spec detection is typed by kind, not by list shape.
;; MarlinResult <- MarlinInput
(def (marlin-import? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-module-import-kind)))

;;; Boundary: Module config detection is typed by kind, not by source shape.
;; MarlinResult <- MarlinInput
(def (marlin-module-config? value)
  (and (object? value)
       (.has? value kind)
       (or (string=? (.get value kind) marlin-modules-kind)
           (string=? (.get value kind) marlin-policy-module-kind))))

;;; Boundary: Policy module detection is typed, not based on source syntax.
;; MarlinResult <- MarlinInput
(def (marlin-policy-module? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-policy-module-kind)))

;;; Boundary: Slot names are the user-facing option ids.
;; MarlinResult <- MarlinInput
(def (marlin-module-slot-option-id slot-name)
  (symbol->string slot-name))

;;; Boundary: Schema specs default to String for concise interface modules.
;; MarlinResult <- MarlinInput
(def (marlin-module-schema-spec-type schema-spec)
  (if (.has? schema-spec type)
    (.get schema-spec type)
    String))

;;; Boundary: Schema metadata is optional user data, not a required constructor arg.
;; MarlinResult <- MarlinInput
(def (marlin-module-schema-spec-metadata schema-spec)
  (if (.has? schema-spec metadata)
    (.get schema-spec metadata)
    '()))

;;; Boundary: Constant/default/optional schema forms are projected from one POO spec.
;; MarlinResult <- MarlinInput
(def (marlin-module-schema-from-spec module-id-value option-id-value schema-spec)
  (let* ((value-type (marlin-module-schema-spec-type schema-spec))
         (metadata-value (marlin-module-schema-spec-metadata schema-spec)))
    (cond
     ((.has? schema-spec constant)
      (make-marlin-deck-runtime-constant-option-schema
       option-id-value
       value-type
       (.get schema-spec constant)
       module-id-value
       metadata-value))
     ((.has? schema-spec default)
      (make-marlin-deck-runtime-defaulted-option-schema
       option-id-value
       value-type
       (.get schema-spec default)
       module-id-value
       metadata-value))
     ((and (.has? schema-spec optional?)
           (.get schema-spec optional?))
      (make-marlin-deck-runtime-optional-option-schema
       option-id-value
       value-type
       module-id-value
       metadata-value))
     (else
      (make-marlin-deck-runtime-required-option-schema
       option-id-value
       value-type
       module-id-value
       metadata-value)))))

;;; Boundary: User option object becomes runtime option configs at the ABI edge.
;; MarlinResult <- MarlinInput
(def (marlin-module-option-configs module)
  (let* ((module-id-value (.get module id))
         (option-object (.get module options)))
    (map (lambda (slot-name)
           (make-marlin-deck-runtime-option-config
            (marlin-module-slot-option-id slot-name)
            (.ref option-object slot-name)
            module-id-value
            '()))
         (.all-slots option-object))))

;;; Boundary: User schema object becomes typed option schemas at the ABI edge.
;; MarlinResult <- MarlinInput
(def (marlin-module-option-schemas module)
  (let* ((module-id-value (.get module id))
         (schema-object (.get module schemas)))
    (map (lambda (slot-name)
           (marlin-module-schema-from-spec
            module-id-value
            (marlin-module-slot-option-id slot-name)
            (.ref schema-object slot-name)))
         (.all-slots schema-object))))

;;; Boundary: Validation matches configs to their schema by option id.
;; MarlinResult <- MarlinInput
(def (marlin-module-find-schema schemas option-id-value)
  (find (lambda (schema)
          (string=? (.get schema id) option-id-value))
        schemas))

;;; Boundary: Missing schemas are typed validation receipts, not runtime crashes.
;; MarlinResult <- MarlinInput
(def (marlin-module-missing-schema-receipt config)
  (make-marlin-deck-runtime-option-validation-receipt
   (.get config id)
   (.get config source-module-id)
   #f
   '("option schema is not declared")
   '()))

;;; Boundary: Validation receipts remain Scheme typed values before Rust projection.
;; MarlinResult <- MarlinInput
(def (marlin-module-option-validation-receipts module)
  (let ((schemas (marlin-module-option-schemas module)))
    (map (lambda (config)
           (let (schema (marlin-module-find-schema schemas (.get config id)))
             (if schema
               (marlin-deck-runtime-option-config-validate schema config)
               (marlin-module-missing-schema-receipt config))))
         (marlin-module-option-configs module))))

;;; Boundary: Imported config objects are the only recursive validation inputs.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-config import-value)
  (cond
   ((marlin-import? import-value)
    (marlin-module-import-config (.get import-value profile)))
   ((marlin-module-config? import-value)
    import-value)
   (else #f)))

;;; Boundary: Validation traversal ignores non-config runtime imports.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-configs module)
  (filter (lambda (import-config) import-config)
          (map marlin-module-import-config (.get module imports))))

;;; Boundary: Full validation receipts are an upstream workflow concern.
;; MarlinResult <- MarlinInput
(def (marlin-module-validation-receipts module)
  (append
   (foldr append
          '()
          (map marlin-module-validation-receipts
               (marlin-module-import-configs module)))
   (marlin-module-option-validation-receipts module)))

;;; Boundary: Imports may be file/profile specs, runtime modules, or configs.
;; MarlinResult <- MarlinInput
(def (marlin-module-runtime-import module)
  (cond
   ((marlin-import? module)
    (marlin-module-runtime-import (.get module profile)))
   ((and (object? module)
         (.has? module kind)
         (or (string=? (.get module kind) marlin-modules-kind)
             (string=? (.get module kind) marlin-policy-module-kind)))
    (marlin-module-apply module))
   (else module)))

;;; Boundary: Public module config is applied once at the runtime projection edge.
;; MarlinResult <- MarlinInput
(def (marlin-module-apply module)
  (make-marlin-deck-runtime-user-module
   (.get module id)
   (map marlin-module-runtime-import (.get module imports))
   (.get module extensions)
   (.get module scripts)
   (marlin-module-option-configs module)
   (.get module metadata)))

;;; Boundary: Evaluation stays a typed Scheme value for native ABI projection.
;; MarlinResult <- MarlinInput
(def (marlin-module-evaluate module)
  (marlin-deck-runtime-user-module-evaluate
   (marlin-module-apply module)))

;;; Boundary: Workflow objects own common runtime projections for user configs.
;; MarlinResult <- MarlinInput
(def (marlin-module-workflow module . maybe-allowed-hook-id-values)
  (let* ((allowed-hook-id-values
          (if (null? maybe-allowed-hook-id-values)
            '()
            (car maybe-allowed-hook-id-values)))
         (runtime-module-value
          (marlin-module-apply module))
         (evaluation-value
          (marlin-deck-runtime-user-module-evaluate runtime-module-value)))
    (.o kind: marlin-module-workflow-kind
        config: module
        runtime-module: runtime-module-value
        evaluation: evaluation-value
        allowed-hook-ids: allowed-hook-id-values
        extension-catalog:
        (marlin-deck-runtime-user-module-extension-catalog
         evaluation-value
         allowed-hook-id-values)
        root-options: (marlin-module-option-configs module)
        option-schemas: (marlin-module-option-schemas module)
        root-validation-receipts:
        (marlin-module-option-validation-receipts module)
        validation-receipts:
        (marlin-module-validation-receipts module))))

;;; Boundary: Catalog lookup is explicit and deterministic.
;; MarlinResult <- MarlinInput
(def (marlin-module-catalog-find catalog module-id-value)
  (find (lambda (module)
          (string=? (.get module id) module-id-value))
        (.get catalog modules)))

;;; Boundary: A missing root id means the first catalog module is the root.
;; MarlinResult <- MarlinInput
(def (marlin-module-catalog-root catalog module-id-value)
  (cond
   (module-id-value
    (or (marlin-module-catalog-find catalog module-id-value)
        (error "marlin module root not found" module-id-value)))
   ((pair? (.get catalog modules))
    (car (.get catalog modules)))
   (else
    (error "marlin module catalog is empty"))))

;;; Boundary: evalModules is the Nix-like user entry backed by POO workflow.
;; MarlinResult <- MarlinInput
(def (marlinEvalModules catalog . eval-options)
  (let* ((root-module-id-value
          (if (null? eval-options) #f (car eval-options)))
         (allowed-hook-id-values
          (if (or (null? eval-options)
                  (null? (cdr eval-options)))
            '()
            (cadr eval-options)))
         (root-module
          (marlin-module-catalog-root catalog root-module-id-value)))
    (if (marlin-policy-module? root-module)
      (let* ((workflow
              (marlin-policy-module-workflow
               root-module
               allowed-hook-id-values))
             (substrate-gate (.get workflow substrate-gate)))
        (.o kind: marlin-eval-modules-result-kind
            catalog-kind: (.get catalog kind)
            root-module-id: (.get root-module id)
            root-module-kind: (.get root-module kind)
            workflow-kind: (.get workflow kind)
            substrate-gate-kind: (.get substrate-gate kind)
            gate-profile: (.get substrate-gate gate-profile)
            projection-target: (.get substrate-gate projection-target)
            receipt-kind: (.get substrate-gate receipt-kind)
            module-evaluation-kind:
            (.get substrate-gate module-evaluation-kind)
            module-count: (.get substrate-gate module-count)
            extension-count: (.get substrate-gate extension-count)
            policy-extension-object-count:
            (.get substrate-gate policy-extension-object-count)
            script-count: (.get substrate-gate script-count)
            option-count: (.get substrate-gate option-count)
            validation-receipt-count:
            (.get substrate-gate validation-receipt-count)
            rust-kernel-owner: (.get substrate-gate rust-kernel-owner)
            scheme-policy-owner: (.get substrate-gate scheme-policy-owner)
            replayable: (.get substrate-gate replayable)))
      (let* ((workflow
              (marlin-module-workflow
               root-module
               allowed-hook-id-values))
             (evaluation-value (.get workflow evaluation)))
        (.o kind: marlin-eval-modules-result-kind
            catalog-kind: (.get catalog kind)
            root-module-id: (.get root-module id)
            root-module-kind: (.get root-module kind)
            workflow-kind: (.get workflow kind)
            substrate-gate-kind: #f
            gate-profile: #f
            projection-target: #f
            receipt-kind: #f
            module-evaluation-kind: (.get evaluation-value kind)
            module-count: (length (.get evaluation-value module-ids))
            extension-count: (length (.get evaluation-value extensions))
            policy-extension-object-count:
            (marlin-policy-extension-object-count
             (.get evaluation-value extensions))
            script-count: (length (.get evaluation-value scripts))
            option-count: (length (.get evaluation-value options))
            validation-receipt-count:
            (length (.get workflow validation-receipts))
            rust-kernel-owner: "rust"
            scheme-policy-owner: "gerbil-poo"
            replayable: #t)))))

;;; Boundary: Full presentation receipt keeps the module system user-facing.
;; MarlinResult <- MarlinInput
(def (marlinModuleSystemPresentation catalog . eval-options)
  (let* ((root-module-id-value
          (if (null? eval-options) #f (car eval-options)))
         (allowed-hook-id-values
          (if (or (null? eval-options)
                  (null? (cdr eval-options)))
            '()
            (cadr eval-options)))
         (root-module
          (marlin-module-catalog-root catalog root-module-id-value))
         (eval-result
          (cond
           ((null? eval-options)
            (marlinEvalModules catalog))
           ((null? (cdr eval-options))
            (marlinEvalModules catalog root-module-id-value))
           (else
            (marlinEvalModules
             catalog
             root-module-id-value
             allowed-hook-id-values)))))
    (.o kind: marlin-module-system-presentation-kind
        catalog-kind: (.get catalog kind)
        catalog-module-count: (length (.get catalog modules))
        root-module-id: (.get root-module id)
        root-module-kind: (.get root-module kind)
        root-import-count: (length (.get root-module imports))
        root-extension-count: (length (.get root-module extensions))
        root-policy-extension-object-count:
        (marlin-policy-extension-object-count
         (.get root-module extensions))
        root-script-count: (length (.get root-module scripts))
        allowed-hook-count: (length allowed-hook-id-values)
        user-entrypoints:
        '("marlinModules"
          "marlinModuleCatalog"
          "marlinEvalModules"
          "marlinModuleSystemPresentation")
        module-eval-result-kind: (.get eval-result kind)
        workflow-kind: (.get eval-result workflow-kind)
        substrate-gate-kind: (.get eval-result substrate-gate-kind)
        projection-target: (.get eval-result projection-target)
        receipt-kind: (.get eval-result receipt-kind)
        module-evaluation-receipt-kind:
        (.get eval-result module-evaluation-kind)
        module-count: (.get eval-result module-count)
        extension-count: (.get eval-result extension-count)
        policy-extension-object-count:
        (.get eval-result policy-extension-object-count)
        script-count: (.get eval-result script-count)
        option-count: (.get eval-result option-count)
        validation-receipt-count:
        (.get eval-result validation-receipt-count)
        projection-chain-kind: marlin-module-projection-chain-kind
        policy-projection-receipt-kind: (.get eval-result kind)
        native-projection-payload-owner: "rust"
        budget-receipt-owner: "rust"
        catalog-resolution-receipt-owner: "rust"
        import-graph-owner: "gerbil-module-system"
        option-merge-owner: "gerbil-poo"
        extension-composition-owner: "gerbil-poo"
        scheme-policy-owner: (.get eval-result scheme-policy-owner)
        rust-kernel-owner: (.get eval-result rust-kernel-owner)
        runtime-lifecycle-owner: "rust"
        rust-parses-scheme-source: #f
        scheme-manufactures-rust-handlers: #f
        replayable: (.get eval-result replayable))))

;;; Boundary: Policy substrate gates prove module evaluation before Rust runtime.
;; MarlinResult <- MarlinInput
(def (marlin-policy-module-substrate-gate policy-module workflow)
  (let (evaluation-value (.get workflow evaluation))
    (.o kind: marlin-policy-substrate-gate-kind
        module-id: (.get policy-module id)
        policy-module-kind: (.get policy-module kind)
        policy-family: (.get policy-module policy-family)
        projection-target: (.get policy-module projection-target)
        receipt-kind: (.get policy-module receipt-kind)
        gate-profile: (.get policy-module gate-profile)
        module-evaluation-kind: (.get evaluation-value kind)
        module-count: (length (.get evaluation-value module-ids))
        extension-count: (length (.get evaluation-value extensions))
        policy-extension-object-count:
        (marlin-policy-extension-object-count
         (.get evaluation-value extensions))
        script-count: (length (.get evaluation-value scripts))
        option-count: (length (.get evaluation-value options))
        validation-receipt-count:
        (length (.get workflow validation-receipts))
        rust-kernel-owner: (.get policy-module rust-kernel-owner)
        scheme-policy-owner: (.get policy-module scheme-policy-owner)
        replayable: (.get policy-module replayable))))

;;; Boundary: Policy workflow wraps the normal module workflow with gate receipt.
;; MarlinResult <- MarlinInput
(def (marlin-policy-module-workflow
      policy-module
      . maybe-allowed-hook-id-values)
  (let (workflow
        (if (null? maybe-allowed-hook-id-values)
          (marlin-module-workflow policy-module)
          (marlin-module-workflow
           policy-module
           (car maybe-allowed-hook-id-values))))
    (.o kind: marlin-policy-module-workflow-kind
        module-id: (.get policy-module id)
        policy-family: (.get policy-module policy-family)
        projection-target: (.get policy-module projection-target)
        policy-extension-object-count:
        (marlin-policy-extension-object-count
         (.get (.get workflow evaluation) extensions))
        extension-catalog: (.get workflow extension-catalog)
        validation-receipt-count:
        (length (.get workflow validation-receipts))
        substrate-gate:
        (marlin-policy-module-substrate-gate policy-module workflow))))
