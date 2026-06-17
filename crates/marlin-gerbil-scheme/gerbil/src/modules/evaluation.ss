;;; -*- Gerbil -*-
;;; Boundary: Module evaluation and Rust-facing projection receipts.

package: modules

(import (only-in :clan/poo/object .all-slots .get .has? .o .ref object?)
        (only-in :clan/poo/type String)
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option
        :modules/kinds
        :modules/core
        :modules/policy-extension
        :modules/policy-module)

(export marlinModuleCatalog
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
        marlin-policy-module-substrate-gate
        marlin-policy-module-workflow)

;;; Boundary: Public catalogs collect typed module values for evaluation.
;; MarlinResult <- MarlinInput
(def (marlinModuleCatalog . module-values)
  (.o kind: marlin-module-catalog-kind
      modules: module-values))

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
