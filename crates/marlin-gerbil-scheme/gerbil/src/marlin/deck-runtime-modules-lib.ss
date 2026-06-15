;;; -*- Gerbil -*-
;;; Boundary: Module owns the public POO module interface for downstream users.

package: marlin

(import :clan/poo/object
        (only-in :clan/poo/type String)
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option)

(export marlin-modules-kind
        marlin-module-workflow-kind
        marlin-module-import-kind
        marlin-import-source-ref-kind
        marlin-import-local-source-kind
        marlinModules
        marlin-import
        marlin-imports
        marlin-imports-append
        marlin-extensions
        marlin-extensions-append
        marlin-import?
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
        marlin-module-workflow)

;;; Boundary: Public user module kind is stable across module-lib helpers.
;; MarlinResult <- MarlinInput
(def marlin-modules-kind
  "marlin.modules.v1")

;;; Boundary: Workflow helpers keep runtime projections out of user config files.
;; MarlinResult <- MarlinInput
(def marlin-module-workflow-kind
  "marlin.modules.workflow.v1")

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
       (string=? (.get value kind) marlin-modules-kind)))

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
         (string=? (.get module kind) marlin-modules-kind))
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
