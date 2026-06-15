;;; -*- Gerbil -*-
;;; Boundary: Module tests POO-driven user option schemas and configs.

(import :clan/poo/mop
        :clan/poo/object
        :clan/poo/type
        :marlin/deck-runtime-user-option
        (only-in :std/sugar ignore-errors)
        :std/test)

;;; Boundary: Schema object mirrors gerbil-poo Slot type/default semantics.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-option-schema user-option-agent-scope-schema
  "agent-scope"
  String
  #t
  #t
  "user-interface-agent"
  #f
  (void)
  "user-module-option-test"
  '((owner . "option-test")))

;;; Boundary: Config object is downstream user data before Rust projection.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-option-config user-option-agent-scope-config
  "agent-scope"
  "user-interface-agent"
  "user-module-option-test"
  '((owner . "option-test")))

;;; Boundary: Invalid config proves type checks use gerbil-poo Type descriptors.
;; MarlinResult <- MarlinInput
(def user-option-invalid-config
  (make-marlin-deck-runtime-option-config
   "agent-scope"
   42
   "user-module-option-test"
   '((owner . "option-test"))))

;;; Boundary: Constant schema proves Slot constant semantics are represented.
;; MarlinResult <- MarlinInput
(def user-option-constant-schema
  (make-marlin-deck-runtime-constant-option-schema
   "entry-mode"
   String
   "script"
   "user-module-option-test"
   '((owner . "option-test"))))

;;; Boundary: Constant config matches its declared schema.
;; MarlinResult <- MarlinInput
(def user-option-constant-config
  (make-marlin-deck-runtime-option-config
   "entry-mode"
   "script"
   "user-module-option-test"
   '((owner . "option-test"))))

;;; Boundary: Constant mismatch stays a typed receipt, not an exception string.
;; MarlinResult <- MarlinInput
(def user-option-constant-invalid-config
  (make-marlin-deck-runtime-option-config
   "entry-mode"
   "inline"
   "user-module-option-test"
   '((owner . "option-test"))))

;;; Boundary: Receipts are inspectable typed POO objects.
;; MarlinResult <- MarlinInput
(def user-option-valid-receipt
  (marlin-deck-runtime-option-config-validate
   user-option-agent-scope-schema
   user-option-agent-scope-config))

;;; Boundary: Invalid type receipt preserves option identity for Rust projection.
;; MarlinResult <- MarlinInput
(def user-option-invalid-receipt
  (marlin-deck-runtime-option-config-validate
   user-option-agent-scope-schema
   user-option-invalid-config))

;;; Boundary: Invalid constant receipt preserves policy-level error detail.
;; MarlinResult <- MarlinInput
(def user-option-constant-invalid-receipt
  (marlin-deck-runtime-option-config-validate
   user-option-constant-schema
   user-option-constant-invalid-config))

;;; Boundary: Contract witness proves generic protocol slots are usable.
;; MarlinResult <- MarlinInput
(def user-option-contract-witness
  (marlin-deck-runtime-option-generic-contract-test-witness))

;;; Boundary: Generic table contract witness is the R017 adapter proof surface.
;; MarlinResult <- MarlinInput
(def (table-contract-tests schema config receipt)
  (let ((schema-type MarlinDeckRuntimeOptionSchema)
        (config-type MarlinDeckRuntimeOptionConfig)
        (receipt-type MarlinDeckRuntimeOptionValidationReceipt))
    (let ((MarlinDeckRuntimeOptionSchema
           (lambda (value) (element? schema-type value)))
          (MarlinDeckRuntimeOptionConfig
           (lambda (value) (element? config-type value)))
          (MarlinDeckRuntimeOptionValidationReceipt
           (lambda (value) (element? receipt-type value))))
      (check (MarlinDeckRuntimeOptionSchema schema) => #t)
      (check (MarlinDeckRuntimeOptionConfig config) => #t)
      (check (MarlinDeckRuntimeOptionValidationReceipt receipt) => #t)
      (check (marlin-deck-runtime-option-schema-table-ref schema 'id)
             => "agent-scope")
      (check (marlin-deck-runtime-option-config-table-ref config 'value)
             => "user-interface-agent")
      (check (marlin-deck-runtime-option-validation-receipt-table-ref
              receipt
              'valid?)
             => #t)
      (check (marlin-deck-runtime-option-schema-table-fold
              (lambda (_slot-name _slot-value count) (+ count 1))
              0
              schema)
             => 10)
      (check (marlin-deck-runtime-option-config-table-fold
              (lambda (_slot-name _slot-value count) (+ count 1))
              0
              config)
             => 5)
      (check (marlin-deck-runtime-option-validation-receipt-table-fold
              (lambda (_slot-name _slot-value count) (+ count 1))
              0
              receipt)
             => 6))))

;;; Boundary: Adapter types are intentionally mentioned in the generic witness owner.
;; MarlinResult <- MarlinInput
(table-contract-tests
 user-option-agent-scope-schema
 user-option-agent-scope-config
 user-option-valid-receipt)

;;; Boundary: Protocol contract tests link parser call facts to POO type descriptors.
;; MarlinResult <- MarlinInput
(def (protocol-contract-tests)
  (ignore-errors (MarlinDeckRuntimeOptionSchema user-option-agent-scope-schema))
  (ignore-errors (MarlinDeckRuntimeOptionConfig user-option-agent-scope-config))
  (ignore-errors
   (MarlinDeckRuntimeOptionValidationReceipt user-option-valid-receipt))
  (check (element? MarlinDeckRuntimeOptionSchema user-option-agent-scope-schema)
         => #t)
  (check (element? MarlinDeckRuntimeOptionConfig user-option-agent-scope-config)
         => #t)
  (check (element?
          MarlinDeckRuntimeOptionValidationReceipt
          user-option-valid-receipt)
         => #t))

;;; Boundary: Generic protocol witness is required by R017 for adapters.
;; MarlinResult <- MarlinInput
(protocol-contract-tests)

(check (.get user-option-agent-scope-schema kind)
       => marlin-deck-runtime-option-schema-kind)
(check (.get user-option-agent-scope-config kind)
       => marlin-deck-runtime-option-config-kind)
(check (.get user-option-valid-receipt kind)
       => marlin-deck-runtime-option-validation-receipt-kind)
(check (.get user-option-valid-receipt valid?) => #t)
(check (.get user-option-valid-receipt errors) => '())
(check (.get user-option-invalid-receipt valid?) => #f)
(check (.get user-option-invalid-receipt errors)
       => '("option value does not satisfy schema type"))
(check (.get
        (marlin-deck-runtime-option-config-validate
         user-option-constant-schema
         user-option-constant-config)
        valid?)
       => #t)
(check (.get user-option-constant-invalid-receipt valid?) => #f)
(check (.get user-option-constant-invalid-receipt errors)
       => '("option value does not match schema constant"))
(check (element? MarlinDeckRuntimeOptionSchema user-option-agent-scope-schema)
       => #t)
(check (element? MarlinDeckRuntimeOptionConfig user-option-agent-scope-config)
       => #t)
(check (element?
        MarlinDeckRuntimeOptionValidationReceipt
        user-option-valid-receipt)
       => #t)
(check (.get user-option-contract-witness schema-valid?) => #t)
(check (.get user-option-contract-witness config-valid?) => #t)
(check (.get user-option-contract-witness receipt-valid?) => #t)
(check (.get user-option-contract-witness schema-equal?) => #t)
(check (.get user-option-contract-witness config-equal?) => #t)
(check (.get user-option-contract-witness receipt-equal?) => #t)
