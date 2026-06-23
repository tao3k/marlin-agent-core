;;; -*- Gerbil -*-
;;; Boundary: Marlin projection receipts over the upstream poo-flow module system.

package: marlin/modules

(import (only-in :clan/poo/object .all-slots .get .has? .o .ref object?)
        (only-in :clan/poo/type String)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-catalog
                 poo-flow-module-value-catalog-find
                 poo-flow-module-value-catalog-root
                 poo-flow-eval-modules
                 poo-flow-module-system-presentation
                 poo-flow-module-workflow
                 poo-flow-module-name
                 poo-flow-module-imports
                 poo-flow-module-import-configs
                 poo-flow-module-schemas
                 poo-flow-module-config
                 poo-flow-module-extensions
                 poo-flow-module-scripts
                 poo-flow-module-system-owner)
        (only-in :marlin/deck-runtime-extension-catalog
                 make-marlin-deck-runtime-extension-catalog
                 marlin-deck-runtime-extension-catalog-add)
        :marlin/deck-runtime-user-option
        :marlin/modules/kinds
        :marlin/modules/policy-extension
        :marlin/modules/policy-module)

(export marlinModuleCatalog
        marlin-module-option-configs
        marlin-module-option-schemas
        marlin-module-find-schema
        marlin-module-missing-schema-receipt
        marlin-module-option-validation-receipts
        marlin-module-validation-receipts
        marlin-module-workflow
        marlin-module-catalog-find
        marlin-module-catalog-root
        marlinEvalModules
        marlinModuleSystemPresentation
        marlin-policy-module-substrate-gate
        marlin-policy-module-workflow)

(def (marlinModuleCatalog . module-values)
  (apply poo-flow-module-catalog module-values))

(def (marlin-module-slot-option-id slot-name)
  (symbol->string slot-name))

(def (marlin-module-schema-spec-type schema-spec)
  (let (schema-type
        (if (.has? schema-spec type)
          (.get schema-spec type)
          String))
    (cond
     ((eq? schema-type 'String) String)
     ((and (string? schema-type)
           (string=? schema-type "String"))
      String)
     (else schema-type))))

(def (marlin-module-schema-spec-metadata schema-spec)
  (if (.has? schema-spec metadata)
    (.get schema-spec metadata)
    '()))

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

(def (marlin-module-option-configs module)
  (let* ((module-id-value (poo-flow-module-name module))
         (option-object (poo-flow-module-config module)))
    (map (lambda (slot-name)
           (make-marlin-deck-runtime-option-config
            (marlin-module-slot-option-id slot-name)
            (.ref option-object slot-name)
            module-id-value
            '()))
         (.all-slots option-object))))

(def (marlin-module-option-schemas module)
  (let* ((module-id-value (poo-flow-module-name module))
         (schema-object (poo-flow-module-schemas module)))
    (map (lambda (slot-name)
           (marlin-module-schema-from-spec
            module-id-value
            (marlin-module-slot-option-id slot-name)
            (.ref schema-object slot-name)))
         (.all-slots schema-object))))

(def (marlin-module-find-schema schemas option-id-value)
  (find (lambda (schema)
          (string=? (.get schema id) option-id-value))
        schemas))

(def (marlin-module-missing-schema-receipt config)
  (make-marlin-deck-runtime-option-validation-receipt
   (.get config id)
   (.get config source-module-id)
   #f
   '("option schema is not declared")
   "missing-schema"
   'unknown-type
   #f
   #f
   #f
   (void)
   #f
   (void)
   "unknown"
   '()))

(def (marlin-module-option-validation-receipts module)
  (let ((schemas (marlin-module-option-schemas module)))
    (map (lambda (config)
           (let (schema (marlin-module-find-schema schemas (.get config id)))
             (if schema
               (marlin-deck-runtime-option-config-validate schema config)
               (marlin-module-missing-schema-receipt config))))
         (marlin-module-option-configs module))))

(def (marlin-module-validation-receipts module)
  (append
   (foldr append
          '()
          (map marlin-module-validation-receipts
               (poo-flow-module-import-configs
                (poo-flow-module-imports module))))
   (marlin-module-option-validation-receipts module)))

(def (marlin-module-extension-catalog evaluation allowed-hook-id-values)
  (foldl (lambda (extension catalog)
           (marlin-deck-runtime-extension-catalog-add catalog extension))
         (make-marlin-deck-runtime-extension-catalog
          allowed-hook-id-values
          '())
         (.get evaluation extensions)))

(def (marlin-module-workflow module . maybe-allowed-hook-id-values)
  (let* ((allowed-hook-id-values
          (if (null? maybe-allowed-hook-id-values)
            '()
            (car maybe-allowed-hook-id-values)))
         (poo-flow-workflow
          (poo-flow-module-workflow module allowed-hook-id-values))
         (evaluation-value
          (.get poo-flow-workflow evaluation)))
    (.o (:: @ (list poo-flow-workflow))
        kind: marlin-module-workflow-kind
        upstream-workflow-kind: (.get poo-flow-workflow kind)
        config: module
        runtime-module: (.get poo-flow-workflow runtime-module)
        evaluation: evaluation-value
        allowed-hook-ids: allowed-hook-id-values
        extension-catalog:
        (marlin-module-extension-catalog evaluation-value allowed-hook-id-values)
        root-options: (marlin-module-option-configs module)
        option-schemas: (marlin-module-option-schemas module)
        root-validation-receipts:
        (marlin-module-option-validation-receipts module)
        validation-receipts:
        (marlin-module-validation-receipts module))))

(def (marlin-module-catalog-find catalog module-id-value)
  (poo-flow-module-value-catalog-find catalog module-id-value))

(def (marlin-module-catalog-root catalog module-id-value)
  (poo-flow-module-value-catalog-root catalog module-id-value))

(def (marlinEvalModules catalog . eval-options)
  (let* ((eval-result (apply poo-flow-eval-modules catalog eval-options))
         (root-module
          (marlin-module-catalog-root
           catalog
           (if (null? eval-options) #f (car eval-options)))))
    (if (marlin-policy-module? root-module)
      (let* ((allowed-hook-id-values
              (if (or (null? eval-options)
                      (null? (cdr eval-options)))
                '()
                (cadr eval-options)))
             (workflow
              (marlin-policy-module-workflow
               root-module
               allowed-hook-id-values))
             (substrate-gate (.get workflow substrate-gate)))
        (.o (:: @ (list eval-result))
            workflow-kind: marlin-policy-module-workflow-kind
            substrate-gate-kind: (.get substrate-gate kind)
            gate-profile: (.get substrate-gate gate-profile)
            projection-target: (.get substrate-gate projection-target)
            receipt-kind: (.get substrate-gate receipt-kind)
            policy-extension-object-count:
            (.get substrate-gate policy-extension-object-count)
            rust-kernel-owner: (.get substrate-gate rust-kernel-owner)
            scheme-policy-owner: (.get substrate-gate scheme-policy-owner)))
      (let* ((allowed-hook-id-values
              (if (or (null? eval-options)
                      (null? (cdr eval-options)))
                '()
                (cadr eval-options)))
             (workflow
              (marlin-module-workflow root-module allowed-hook-id-values))
             (evaluation-value (.get workflow evaluation)))
        (.o (:: @ (list eval-result))
            substrate-gate-kind: #f
            gate-profile: #f
            projection-target: #f
            receipt-kind: #f
            policy-extension-object-count:
            (marlin-policy-extension-object-count
             (.get evaluation-value extensions))
            rust-kernel-owner: (.ref eval-result 'runtime-owner)
            scheme-policy-owner: (.ref eval-result 'scheme-owner))))))

(def (marlinModuleSystemPresentation catalog . eval-options)
  (let* ((presentation
          (apply poo-flow-module-system-presentation catalog eval-options))
         (eval-result
          (apply marlinEvalModules catalog eval-options)))
    (.o (:: @ (list presentation))
        user-entrypoints:
        '("marlinModules"
          "marlinModuleCatalog"
          "marlinEvalModules"
          "marlinModuleSystemPresentation")
        substrate-gate-kind:
        (if (.has? eval-result substrate-gate-kind)
          (.get eval-result substrate-gate-kind)
          #f)
        projection-target:
        (if (.has? eval-result projection-target)
          (.get eval-result projection-target)
          #f)
        receipt-kind:
        (if (.has? eval-result receipt-kind)
          (.get eval-result receipt-kind)
          #f)
        projection-chain-kind: marlin-module-projection-chain-kind
        policy-projection-receipt-kind: (.ref eval-result 'kind)
        import-graph-owner: poo-flow-module-system-owner
        option-policy-owner: poo-flow-module-system-owner
        option-merge-owner: poo-flow-module-system-owner
        extension-composition-owner: poo-flow-module-system-owner
        native-projection-payload-owner: "rust"
        budget-receipt-owner: "rust"
        catalog-resolution-receipt-owner: "rust"
        policy-extension-object-count:
        (marlin-policy-extension-object-count
         (poo-flow-module-extensions
          (marlin-module-catalog-root
           catalog
           (if (null? eval-options) #f (car eval-options)))))
        root-policy-extension-object-count:
        (marlin-policy-extension-object-count
         (poo-flow-module-extensions
          (marlin-module-catalog-root
           catalog
           (if (null? eval-options) #f (car eval-options)))))
        scheme-policy-owner:
        (if (.has? eval-result scheme-policy-owner)
          (.get eval-result scheme-policy-owner)
          (.ref eval-result 'scheme-owner))
        rust-kernel-owner:
        (if (.has? eval-result rust-kernel-owner)
          (.get eval-result rust-kernel-owner)
          (.ref eval-result 'runtime-owner))
        rust-parses-scheme-source: #f
        scheme-manufactures-rust-handlers: #f)))

(def (marlin-policy-module-substrate-gate policy-module workflow)
  (let (evaluation-value (.get workflow evaluation))
    (.o kind: marlin-policy-substrate-gate-kind
        module-id: (poo-flow-module-name policy-module)
        policy-module-kind: (.get policy-module policy-module-kind)
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
        module-id: (poo-flow-module-name policy-module)
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
