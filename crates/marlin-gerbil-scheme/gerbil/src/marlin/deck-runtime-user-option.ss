;;; -*- Gerbil -*-
;;; Boundary: Module owns POO-driven user option schemas for downstream modules.

package: marlin

(import (only-in :clan/poo/object .get .has? .ref object? .o)
        (only-in :clan/poo/mop
                 define-type
                 Class.
                 .new
                 Type
                 Any
                 Bool
                 validate
                 element?
                 raise-type-error
                 sexp<-)
        (only-in :clan/poo/type String List)
        :clan/poo/brace)

(export marlin-deck-runtime-option-schema-kind
        marlin-deck-runtime-option-config-kind
        marlin-deck-runtime-option-validation-receipt-kind
        MarlinDeckRuntimeOptionSchema
        MarlinDeckRuntimeOptionConfig
        MarlinDeckRuntimeOptionValidationReceipt
        make-marlin-deck-runtime-option-schema
        make-marlin-deck-runtime-required-option-schema
        make-marlin-deck-runtime-optional-option-schema
        make-marlin-deck-runtime-defaulted-option-schema
        make-marlin-deck-runtime-constant-option-schema
        defmarlin-deck-runtime-option-schema
        make-marlin-deck-runtime-option-config
        defmarlin-deck-runtime-option-config
        make-marlin-deck-runtime-option-validation-receipt
        marlin-deck-runtime-option-schema-table-keys
        marlin-deck-runtime-option-schema-table-ref
        marlin-deck-runtime-option-schema-table-alist
        marlin-deck-runtime-option-schema-table-fold
        marlin-deck-runtime-option-config-table-keys
        marlin-deck-runtime-option-config-table-ref
        marlin-deck-runtime-option-config-table-alist
        marlin-deck-runtime-option-config-table-fold
        marlin-deck-runtime-option-validation-receipt-table-keys
        marlin-deck-runtime-option-validation-receipt-table-ref
        marlin-deck-runtime-option-validation-receipt-table-alist
        marlin-deck-runtime-option-validation-receipt-table-fold
        marlin-deck-runtime-option-schema-generic-contract-test-witness
        marlin-deck-runtime-option-config-generic-contract-test-witness
        marlin-deck-runtime-option-validation-receipt-generic-contract-test-witness
        marlin-deck-runtime-option-generic-contract-test-witness
        marlin-deck-runtime-option-schema-match?
        marlin-deck-runtime-option-config-match?
        marlin-deck-runtime-option-config-validate)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-option-schema-kind
  "marlin-deck-runtime.option-schema.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-option-config-kind
  "marlin-deck-runtime.option-config.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-option-validation-receipt-kind
  "marlin-deck-runtime.option-validation-receipt.v1")

;;; Boundary: Type projection avoids recursively serializing dependency internals.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-type-label type-value)
  (if (and (object? type-value) (.has? type-value sexp))
    (.get type-value sexp)
    'unknown-type))

;;; Boundary: Table keys expose the schema protocol without ad hoc alists.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-schema-table-keys _schema)
  '(kind id value-type optional? has-default? default has-constant? constant owner metadata))

;;; Boundary: Table ref is the dynamic lookup surface for schema protocol slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-schema-table-ref schema slot-name)
  (.ref schema slot-name))

;;; Boundary: Table alist is a shallow projection, not text serialization.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-schema-table-alist schema)
  `((kind . ,(.get schema kind))
    (id . ,(.get schema id))
    (value-type . ,(marlin-deck-runtime-option-type-label (.get schema value-type)))
    (optional? . ,(.get schema optional?))
    (has-default? . ,(.get schema has-default?))
    (default . ,(.get schema default))
    (has-constant? . ,(.get schema has-constant?))
    (constant . ,(.get schema constant))
    (owner . ,(.get schema owner))
    (metadata . ,(.get schema metadata))))

;;; Boundary: Fold keeps table traversal typed and deterministic.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-schema-table-fold step seed schema)
  (foldl (lambda (entry acc) (step (car entry) (cdr entry) acc))
         seed
         (marlin-deck-runtime-option-schema-table-alist schema)))

;;; Boundary: Table keys expose config protocol slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-table-keys _config)
  '(kind id value source-module-id metadata))

;;; Boundary: Table ref is the dynamic lookup surface for config protocol slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-table-ref config slot-name)
  (.ref config slot-name))

;;; Boundary: Config alist projection stays shallow Scheme data.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-table-alist config)
  `((kind . ,(.get config kind))
    (id . ,(.get config id))
    (value . ,(.get config value))
    (source-module-id . ,(.get config source-module-id))
    (metadata . ,(.get config metadata))))

;;; Boundary: Config fold keeps table traversal typed and deterministic.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-table-fold step seed config)
  (foldl (lambda (entry acc) (step (car entry) (cdr entry) acc))
         seed
         (marlin-deck-runtime-option-config-table-alist config)))

;;; Boundary: Receipt keys expose validation protocol slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-validation-receipt-table-keys _receipt)
  '(kind option-id source-module-id valid? errors metadata))

;;; Boundary: Table ref is the dynamic lookup surface for receipt protocol slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-validation-receipt-table-ref receipt slot-name)
  (.ref receipt slot-name))

;;; Boundary: Receipt alist projection stays shallow Scheme data.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-validation-receipt-table-alist receipt)
  `((kind . ,(.get receipt kind))
    (option-id . ,(.get receipt option-id))
    (source-module-id . ,(.get receipt source-module-id))
    (valid? . ,(.get receipt valid?))
    (errors . ,(.get receipt errors))
    (metadata . ,(.get receipt metadata))))

;;; Boundary: Receipt fold keeps table traversal typed and deterministic.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-validation-receipt-table-fold step seed receipt)
  (foldl (lambda (entry acc) (step (car entry) (cdr entry) acc))
         seed
         (marlin-deck-runtime-option-validation-receipt-table-alist receipt)))

;;; Boundary: Type descriptor mirrors gerbil-poo Slot semantics for user options.
;; MarlinResult <- MarlinInput
(define-type (MarlinDeckRuntimeOptionSchema @ Class.)
  slots: =>.+
  {kind: {type: String constant: marlin-deck-runtime-option-schema-kind}
   id: {type: String}
   value-type: {type: Type}
   optional?: {type: Bool default: #f}
   has-default?: {type: Bool default: #f}
   default: {type: Any optional: #t}
   has-constant?: {type: Bool default: #f}
   constant: {type: Any optional: #t}
   owner: {type: String default: "user"}
   metadata: {type: Any default: '()}}
  protocol-slot-surface:
  '(kind id value-type optional? has-default? default has-constant? constant owner metadata)
  table-method-surface:
  '(.ref .foldl .foldr)
  derived-protocol-capability:
  '(typed-validation sexp-conversion json-conversion equality)
  .ref:
  marlin-deck-runtime-option-schema-table-ref
  .foldl:
  marlin-deck-runtime-option-schema-table-fold
  .foldr:
  marlin-deck-runtime-option-schema-table-fold
  .validate:
  (lambda (x)
    (if (element? @ x) x (raise-type-error @ x)))
  .sexp<-:
  (lambda (x)
    `(option-schema
      id: ,(.get x id)
      value-type: ,(marlin-deck-runtime-option-type-label (.get x value-type))
      optional?: ,(.get x optional?)
      has-default?: ,(.get x has-default?)
      default: ,(.get x default)
      has-constant?: ,(.get x has-constant?)
      constant: ,(.get x constant)
      owner: ,(.get x owner)
      metadata: ,(.get x metadata)))
  .json<-:
  (lambda (x)
    `((kind . ,(.get x kind))
      (id . ,(.get x id))
      (valueType . ,(marlin-deck-runtime-option-type-label (.get x value-type)))
      (optional . ,(.get x optional?))
      (hasDefault . ,(.get x has-default?))
      (default . ,(.get x default))
      (hasConstant . ,(.get x has-constant?))
      (constant . ,(.get x constant))
      (owner . ,(.get x owner))
      (metadata . ,(.get x metadata))))
  .=?:
  (lambda (left right)
    (and (string=? (.get left id) (.get right id))
         (eq? (.get left value-type) (.get right value-type))
         (equal? (.get left optional?) (.get right optional?))
         (equal? (.get left has-default?) (.get right has-default?))
         (equal? (.get left default) (.get right default))
         (equal? (.get left has-constant?) (.get right has-constant?))
         (equal? (.get left constant) (.get right constant))))
  sealed: #t)

;;; Boundary: User config stays a typed POO object before native ABI projection.
;; MarlinResult <- MarlinInput
(define-type (MarlinDeckRuntimeOptionConfig @ Class.)
  slots: =>.+
  {kind: {type: String constant: marlin-deck-runtime-option-config-kind}
   id: {type: String}
   value: {type: Any}
   source-module-id: {type: String}
   metadata: {type: Any default: '()}}
  protocol-slot-surface:
  '(kind id value source-module-id metadata)
  table-method-surface:
  '(.ref .foldl .foldr)
  derived-protocol-capability:
  '(typed-validation sexp-conversion json-conversion equality)
  .ref:
  marlin-deck-runtime-option-config-table-ref
  .foldl:
  marlin-deck-runtime-option-config-table-fold
  .foldr:
  marlin-deck-runtime-option-config-table-fold
  .validate:
  (lambda (x)
    (if (element? @ x) x (raise-type-error @ x)))
  .sexp<-:
  (lambda (x)
    `(option-config
      id: ,(.get x id)
      value: ,(.get x value)
      source-module-id: ,(.get x source-module-id)
      metadata: ,(.get x metadata)))
  .json<-:
  (lambda (x)
    `((kind . ,(.get x kind))
      (id . ,(.get x id))
      (value . ,(.get x value))
      (sourceModuleId . ,(.get x source-module-id))
      (metadata . ,(.get x metadata))))
  .=?:
  (lambda (left right)
    (and (string=? (.get left id) (.get right id))
         (equal? (.get left value) (.get right value))
         (string=? (.get left source-module-id)
                   (.get right source-module-id))))
  sealed: #t)

;;; Boundary: Validation result is a Scheme typed receipt, not text serialization.
;; MarlinResult <- MarlinInput
(define-type (MarlinDeckRuntimeOptionValidationReceipt @ Class.)
  slots: =>.+
  {kind: {type: String constant: marlin-deck-runtime-option-validation-receipt-kind}
   option-id: {type: String}
   source-module-id: {type: String}
   valid?: {type: Bool}
   errors: {type: (List String)}
   metadata: {type: Any default: '()}}
  protocol-slot-surface:
  '(kind option-id source-module-id valid? errors metadata)
  table-method-surface:
  '(.ref .foldl .foldr)
  derived-protocol-capability:
  '(typed-validation sexp-conversion json-conversion equality)
  .ref:
  marlin-deck-runtime-option-validation-receipt-table-ref
  .foldl:
  marlin-deck-runtime-option-validation-receipt-table-fold
  .foldr:
  marlin-deck-runtime-option-validation-receipt-table-fold
  .validate:
  (lambda (x)
    (if (element? @ x) x (raise-type-error @ x)))
  .sexp<-:
  (lambda (x)
    `(option-validation-receipt
      option-id: ,(.get x option-id)
      source-module-id: ,(.get x source-module-id)
      valid?: ,(.get x valid?)
      errors: ,(.get x errors)
      metadata: ,(.get x metadata)))
  .json<-:
  (lambda (x)
    `((kind . ,(.get x kind))
      (optionId . ,(.get x option-id))
      (sourceModuleId . ,(.get x source-module-id))
      (valid . ,(.get x valid?))
      (errors . ,(.get x errors))
      (metadata . ,(.get x metadata))))
  .=?:
  (lambda (left right)
    (and (string=? (.get left option-id) (.get right option-id))
         (string=? (.get left source-module-id)
                   (.get right source-module-id))
         (equal? (.get left valid?) (.get right valid?))
         (equal? (.get left errors) (.get right errors))))
  sealed: #t)

;;; Boundary: Schema constructor validates the schema descriptor itself.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-option-schema
      option-id-value
      value-type-value
      optional-value
      has-default-value
      default-value
      has-constant-value
      constant-value
      owner-value
      metadata-value)
  (let (schema
        (.new MarlinDeckRuntimeOptionSchema
              (id option-id-value)
              (value-type (validate Type value-type-value))
              (optional? optional-value)
              (has-default? has-default-value)
              (default default-value)
              (has-constant? has-constant-value)
              (constant constant-value)
              (owner owner-value)
              (metadata metadata-value)))
    (validate MarlinDeckRuntimeOptionSchema schema)
    schema))

;;; Boundary: Required schema is explicit so absence does not become ambiguous.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-required-option-schema
      option-id-value
      value-type-value
      owner-value
      metadata-value)
  (make-marlin-deck-runtime-option-schema
   option-id-value
   value-type-value
   #f
   #f
   (void)
   #f
   (void)
   owner-value
   metadata-value))

;;; Boundary: Optional schema records gerbil-poo optional slot semantics.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-optional-option-schema
      option-id-value
      value-type-value
      owner-value
      metadata-value)
  (make-marlin-deck-runtime-option-schema
   option-id-value
   value-type-value
   #t
   #f
   (void)
   #f
   (void)
   owner-value
   metadata-value))

;;; Boundary: Defaulted schema carries the slot default value as typed Scheme data.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-defaulted-option-schema
      option-id-value
      value-type-value
      default-value
      owner-value
      metadata-value)
  (make-marlin-deck-runtime-option-schema
   option-id-value
   value-type-value
   #t
   #t
   default-value
   #f
   (void)
   owner-value
   metadata-value))

;;; Boundary: Constant schema mirrors gerbil-poo Slot constant semantics.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-constant-option-schema
      option-id-value
      value-type-value
      constant-value
      owner-value
      metadata-value)
  (make-marlin-deck-runtime-option-schema
   option-id-value
   value-type-value
   #f
   #f
   (void)
   #t
   constant-value
   owner-value
   metadata-value))

;;; Boundary: Macro is the user-facing schema declaration surface.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-option-schema ()
  ((_ binding
      option-id
      value-type
      optional?
      has-default?
      default
      has-constant?
      constant
      owner
      metadata)
   (def binding
     (make-marlin-deck-runtime-option-schema
      option-id
      value-type
      optional?
      has-default?
      default
      has-constant?
      constant
      owner
      metadata))))

;;; Boundary: Config constructor validates only the config object shape.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-option-config
      option-id-value
      option-value
      source-module-id-value
      metadata-value)
  (let (config
        (.new MarlinDeckRuntimeOptionConfig
              (id option-id-value)
              (value option-value)
              (source-module-id source-module-id-value)
              (metadata metadata-value)))
    (validate MarlinDeckRuntimeOptionConfig config)
    config))

;;; Boundary: Macro is the user-facing config declaration surface.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-option-config ()
  ((_ binding
      option-id
      value
      source-module-id
      metadata)
   (def binding
     (make-marlin-deck-runtime-option-config
      option-id
      value
      source-module-id
      metadata))))

;;; Boundary: Receipt constructor keeps validation output typed and inspectable.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-option-validation-receipt
      option-id-value
      source-module-id-value
      valid-value
      error-values
      metadata-value)
  (let (receipt
        (.new MarlinDeckRuntimeOptionValidationReceipt
              (option-id option-id-value)
              (source-module-id source-module-id-value)
              (valid? valid-value)
              (errors error-values)
              (metadata metadata-value)))
    (validate MarlinDeckRuntimeOptionValidationReceipt receipt)
    receipt))

;;; Boundary: Schema witness proves table, equality, and validation contract slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-schema-generic-contract-test-witness schema)
  (.o valid?: (element? MarlinDeckRuntimeOptionSchema schema)
      conversion: (marlin-deck-runtime-option-schema-table-alist schema)
      self-equal?:
      ((.get MarlinDeckRuntimeOptionSchema .=?) schema schema)
      table-keys: (marlin-deck-runtime-option-schema-table-keys schema)
      table-alist: (marlin-deck-runtime-option-schema-table-alist schema)
      table-count:
      (marlin-deck-runtime-option-schema-table-fold
       (lambda (_slot-name _slot-value count) (+ count 1))
       0
       schema)))

;;; Boundary: Config witness proves table, equality, and validation contract slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-generic-contract-test-witness config)
  (.o valid?: (element? MarlinDeckRuntimeOptionConfig config)
      conversion: (marlin-deck-runtime-option-config-table-alist config)
      self-equal?:
      ((.get MarlinDeckRuntimeOptionConfig .=?) config config)
      table-keys: (marlin-deck-runtime-option-config-table-keys config)
      table-alist: (marlin-deck-runtime-option-config-table-alist config)
      table-count:
      (marlin-deck-runtime-option-config-table-fold
       (lambda (_slot-name _slot-value count) (+ count 1))
       0
       config)))

;;; Boundary: Receipt witness proves table, equality, and validation contract slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-validation-receipt-generic-contract-test-witness
      receipt)
  (.o valid?: (element? MarlinDeckRuntimeOptionValidationReceipt receipt)
      conversion:
      (marlin-deck-runtime-option-validation-receipt-table-alist receipt)
      self-equal?:
      ((.get MarlinDeckRuntimeOptionValidationReceipt .=?) receipt receipt)
      table-keys:
      (marlin-deck-runtime-option-validation-receipt-table-keys receipt)
      table-alist:
      (marlin-deck-runtime-option-validation-receipt-table-alist receipt)
      table-count:
      (marlin-deck-runtime-option-validation-receipt-table-fold
       (lambda (_slot-name _slot-value count) (+ count 1))
       0
       receipt)))

;;; Boundary: Contract witness exercises generic validation/conversion/equality slots.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-generic-contract-test-witness)
  (let* ((schema
          (make-marlin-deck-runtime-required-option-schema
           "contract-option"
           String
           "contract-witness"
           '((owner . "contract-witness"))))
         (config
          (make-marlin-deck-runtime-option-config
           "contract-option"
           "enabled"
           "contract-witness"
           '((owner . "contract-witness"))))
         (receipt
          (marlin-deck-runtime-option-config-validate schema config))
         (schema-witness
          (marlin-deck-runtime-option-schema-generic-contract-test-witness
           schema))
         (config-witness
          (marlin-deck-runtime-option-config-generic-contract-test-witness
           config))
         (receipt-witness
          (marlin-deck-runtime-option-validation-receipt-generic-contract-test-witness
           receipt)))
    (.o kind: "marlin-deck-runtime.option-contract-witness.v1"
        schema-valid?: (.get schema-witness valid?)
        config-valid?: (.get config-witness valid?)
        receipt-valid?: (.get receipt-witness valid?)
        schema-table-count: (.get schema-witness table-count)
        config-table-count: (.get config-witness table-count)
        receipt-table-count: (.get receipt-witness table-count)
        schema-equal?: (.get schema-witness self-equal?)
        config-equal?: (.get config-witness self-equal?)
        receipt-equal?: (.get receipt-witness self-equal?))))

;;; Boundary: Schema matching stays in Scheme POO policy logic.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-schema-match? schema config)
  (string=? (.get schema id) (.get config id)))

;;; Boundary: Config matching is the module merge identity.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-match? left right)
  (string=? (.get left id) (.get right id)))

;;; Boundary: Validation applies gerbil-poo Type semantics to user values.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-option-config-validate schema config)
  (let* ((option-id-value (.get config id))
         (source-module-id-value (.get config source-module-id))
         (value (.get config value))
         (value-type (.get schema value-type))
         (type-valid? (element? value-type value))
         (constant-valid?
          (or (not (.get schema has-constant?))
              (equal? value (.get schema constant))))
         (valid? (and (marlin-deck-runtime-option-schema-match? schema config)
                      type-valid?
                      constant-valid?))
         (errors
          (append
           (if (marlin-deck-runtime-option-schema-match? schema config)
             '()
             (list "option id does not match schema"))
           (if type-valid?
             '()
             (list "option value does not satisfy schema type"))
           (if constant-valid?
             '()
             (list "option value does not match schema constant")))))
    (make-marlin-deck-runtime-option-validation-receipt
     option-id-value
     source-module-id-value
     valid?
     errors
     '())))
