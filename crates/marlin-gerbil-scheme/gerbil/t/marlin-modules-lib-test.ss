;;; -*- Gerbil -*-
;;; Boundary: Module tests the Marlin adapter over upstream poo-flow modules.

(import :clan/poo/object
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-interface
                 poo-flow-module-name
                 poo-flow-string-constant
                 poo-flow-modules
                 poo-flow-import
                 poo-flow-imports
                 poo-flow-imports-append
                 poo-flow-module-option-config-id
                 poo-flow-module-system-owner
                 poo-flow-module-source-ref-kind
                 poo-flow-module-source-ref-metadata
                 poo-flow-module-source-ref-value)
        :marlin/modules/evaluation
        :marlin/modules/lib)

;;; Boundary: Local checks stay silent around POO-heavy values.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

;;; Boundary: User config stays concise and POO-driven like a module attrset.
;; MarlinResult <- MarlinInput
(def modules-lib-base-interface
  (poo-flow-module-interface
   "ModulesLibBase"
   (.o layer: (poo-flow-string-constant "base"))
   '((owner . "modules-lib-test"))))

;;; Boundary: User config references an imported interface instead of schemas.
;; MarlinResult <- MarlinInput
(def modules-lib-base
  (poo-flow-modules
   modules-lib-base-interface
   (.o id: "modules-lib-base"
       config:
       (.o layer: "base"))))

;;; Boundary: Root interface is separate from the user config record.
;; MarlinResult <- MarlinInput
(def modules-lib-root-interface
  (poo-flow-module-interface
   "ModulesLibRoot"
   (.o surface: (poo-flow-string-constant "example")
       entry: (poo-flow-string-constant "workflow"))
   '((owner . "modules-lib-test"))))

;;; Boundary: Root config imports another poo-flow module object directly.
;; MarlinResult <- MarlinInput
(def modules-lib-root-base
  (poo-flow-modules
   modules-lib-root-interface
   (.o id: "modules-lib-root"
       config:
       (.o surface: "example"
           entry: "workflow"))))

;;; Boundary: Root config extends imports through native POO slot composition.
;; MarlinResult <- MarlinInput
(def modules-lib-root
  (.o (:: @ (list modules-lib-root-base))
      (imports => poo-flow-imports-append
               (poo-flow-imports
                (poo-flow-import "./base.ss" modules-lib-base)
                (poo-flow-import modules-lib-base)))))

;;; Boundary: Workflow helper owns common runtime projections.
;; MarlinResult <- MarlinInput
(def modules-lib-workflow
  (marlin-module-workflow modules-lib-root))

;;; Boundary: Catalog is the public collection input for evalModules.
;; MarlinResult <- MarlinInput
(def modules-lib-catalog
  (marlinModuleCatalog modules-lib-root))

;;; Boundary: evalModules returns scalar receipts for Rust projection.
;; MarlinResult <- MarlinInput
(def modules-lib-eval-result
  (marlinEvalModules modules-lib-catalog "modules-lib-root" '()))

;;; Boundary: Presentation shows the complete module-system contract.
;; MarlinResult <- MarlinInput
(def modules-lib-presentation
  (marlinModuleSystemPresentation
   modules-lib-catalog
   "modules-lib-root"
   '()))

;;; Boundary: Tests inspect workflow slots instead of user-facing config helpers.
;; MarlinResult <- MarlinInput
(def modules-lib-root-options
  (.get modules-lib-workflow root-options))

;;; Boundary: Runtime module is materialized by the workflow helper.
;; MarlinResult <- MarlinInput
(def modules-lib-runtime-module
  (.get modules-lib-workflow runtime-module))

;;; Boundary: Evaluation proves imported poo-flow modules are applied recursively.
;; MarlinResult <- MarlinInput
(def modules-lib-evaluation
  (.get modules-lib-workflow evaluation))

;;; Boundary: Missing schemas return typed receipts instead of runtime failures.
;; MarlinResult <- MarlinInput
(def modules-lib-empty-interface
  (poo-flow-module-interface
   "ModulesLibEmpty"
   (.o)
   '((owner . "modules-lib-test"))))

;;; Boundary: Missing schemas return typed receipts instead of runtime failures.
;; MarlinResult <- MarlinInput
(def modules-lib-missing-schema
  (poo-flow-modules
   modules-lib-empty-interface
   (.o id: "modules-lib-missing-schema"
       config:
       (.o orphan: "value"))))

(def modules-lib-missing-schema-receipt
  (car
   (.get (marlin-module-workflow modules-lib-missing-schema)
         root-validation-receipts)))

;;; Boundary: Policy objects are POO prefab furniture managed by Scheme.
;; MarlinResult <- MarlinInput
(def modules-lib-default-route-object
  (marlinPolicyObject
   "model-route-policy"
   "default-route"
   (.o provider: "openai"
       model: "gpt-5.4"
       route: "interactive")
   '((owner . "modules-lib-test"))))

;;; Boundary: Review policy can be disabled without disappearing from receipts.
;; MarlinResult <- MarlinInput
(def modules-lib-human-review-object
  (marlinPolicyObject
   "human-review-policy"
   "human-review"
   (.o trigger: "high-risk-tool"
       reviewer: "root-agent")
   '((owner . "modules-lib-test"))))

;;; Boundary: Hook policy object names an existing Rust catalog handler id.
;; MarlinResult <- MarlinInput
(def modules-lib-hook-object
  (marlinPolicyObject
   "hook-selection-policy"
   "runtime-hook"
   (.o hook-id: "runtime-catalog-hook"
       action: "register")
   '((owner . "modules-lib-test"))))

;;; Boundary: Added objects are regular POO values wrapped by policy metadata.
;; MarlinResult <- MarlinInput
(def modules-lib-memory-trigger-object
  (marlinMemoryTriggerPolicy
   "memory-trigger"
   (.o trigger: "context-pressure"
       action: "compact")
   '((owner . "modules-lib-test"))))

;;; Boundary: Replacement keeps Rust handler ownership outside Scheme.
;; MarlinResult <- MarlinInput
(def modules-lib-fast-route-object
  (marlinPolicyObject
   "model-route-policy"
   "fast-route"
   (.o provider: "openai"
       model: "gpt-5.4-mini"
       route: "fast")
   '((owner . "modules-lib-test"))))

;;; Boundary: Prefab pack starts as a furnished module and applies surgery.
;; MarlinResult <- MarlinInput
(defmarlin-policy-pack modules-lib-policy-pack
  (id "modules-lib-prefab-pack")
  (module modules-lib-root)
  (policy-objects modules-lib-default-route-object
                  modules-lib-human-review-object
                  modules-lib-hook-object)
  (object-operations
   (marlin-add-object
    modules-lib-memory-trigger-object
    "ship memory trigger as prefab furniture")
   (marlin-remove-object
    "hook-selection-policy"
    "runtime-hook"
    "runtime hook handler stays in Rust catalog")
   (marlin-disable-object
    "human-review-policy"
    "human-review"
    "disabled by customer object surgery")
   (marlin-replace-object
    "model-route-policy"
    "default-route"
    modules-lib-fast-route-object
    "replace route object without Rust merge logic"))
  (allowed-hook-ids)
  (metadata '((owner . "modules-lib-test") (surface . "prefab-pack"))))

;;; Boundary: Pack catalogs are first-class policy bundle collections.
;; MarlinResult <- MarlinInput
(def modules-lib-policy-pack-catalog
  (marlinPackCatalog modules-lib-policy-pack))

;;; Boundary: Tests find post-surgery policy objects by family-local id.
;; MarlinResult <- MarlinInput
(def (modules-lib-policy-pack-object object-id-value)
  (let (matches
        (filter (lambda (policy-object)
                  (and (marlin-policy-object? policy-object)
                       (string=? (marlin-policy-object-id policy-object)
                                 object-id-value)))
                (.get modules-lib-policy-pack policy-objects)))
    (if (pair? matches)
      (car matches)
      #f)))

(check (.get modules-lib-root kind) => marlin-modules-kind)
(check (.get modules-lib-workflow kind) => marlin-module-workflow-kind)
(check (.get modules-lib-catalog kind) => marlin-module-catalog-kind)
(check (length (.get modules-lib-catalog modules)) => 1)
(check (.get modules-lib-eval-result kind)
       => marlin-eval-modules-result-kind)
(check (.get modules-lib-eval-result root-module-id)
       => "modules-lib-root")
(check (.get modules-lib-eval-result workflow-kind)
       => marlin-module-workflow-kind)
(check (.get modules-lib-eval-result module-count) => 2)
(check (.get modules-lib-eval-result option-count) => 3)
(check (.get modules-lib-eval-result validation-receipt-count) => 3)
(check (.get modules-lib-eval-result policy-extension-object-count) => 0)
(check (.get modules-lib-presentation kind)
       => marlin-module-system-presentation-kind)
(check (.get modules-lib-presentation projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get modules-lib-presentation import-graph-owner)
       => poo-flow-module-system-owner)
(check (.get modules-lib-presentation option-policy-owner)
       => poo-flow-module-system-owner)
(check (.get modules-lib-presentation extension-composition-owner)
       => poo-flow-module-system-owner)
(check (.get modules-lib-presentation native-projection-payload-owner)
       => "rust")
(check (.get modules-lib-presentation rust-parses-scheme-source)
       => #f)
(check (.get modules-lib-presentation scheme-manufactures-rust-handlers)
       => #f)
(check (.get (car (.get modules-lib-root imports)) kind)
       => marlin-module-import-kind)
(check (poo-flow-module-source-ref-kind
        (.get (car (.get modules-lib-root imports)) source-ref))
       => 'local)
(check (cdr
        (assq 'kind
              (poo-flow-module-source-ref-metadata
               (.get (car (.get modules-lib-root imports)) source-ref))))
       => marlin-import-local-source-kind)
(check (poo-flow-module-source-ref-value
        (.get (car (.get modules-lib-root imports)) source-ref))
       => "./base.ss")
(check (.get (cadr (.get modules-lib-root imports)) source-ref) => #f)
(check (poo-flow-module-name
        (.get (cadr (.get modules-lib-root imports)) profile))
       => "modules-lib-base")
(check (map (lambda (option) (.get option id))
            modules-lib-root-options)
       => '("surface" "entry"))
(check (map (lambda (schema) (.get schema id))
            (marlin-module-option-schemas modules-lib-root))
       => '("surface" "entry"))
(check (map (lambda (receipt) (.get receipt valid?))
            (marlin-module-option-validation-receipts modules-lib-root))
       => '(#t #t))
(check (.get modules-lib-runtime-module id) => "modules-lib-root")
(check (.get modules-lib-evaluation module-ids)
       => '("modules-lib-root" "modules-lib-base"))
(check (map poo-flow-module-option-config-id
            (.get modules-lib-evaluation options))
       => '("surface" "entry" "layer"))
(check (.get modules-lib-missing-schema-receipt valid?) => #f)
(unless (= (length (.get modules-lib-missing-schema-receipt errors)) 1)
  (error "expected one missing schema error"))
(unless (string=? (car (.get modules-lib-missing-schema-receipt errors))
                  "option schema is not declared")
  (error "unexpected missing schema error"))
(check (.get modules-lib-missing-schema-receipt contract-kind)
       => "missing-schema")
(check (.get modules-lib-missing-schema-receipt value-type-label)
       => 'unknown-type)
(check (.get modules-lib-missing-schema-receipt schema-owner)
       => "unknown")

;;; Boundary: POO-heavy assertions stay scalar to avoid verbose object printing.
;; MarlinResult <- MarlinInput
(def (modules-lib-assert-equal actual expected message)
  (unless (equal? actual expected)
    (error message actual expected)))

(modules-lib-assert-equal
 (marlin-policy-object? modules-lib-default-route-object)
 #t
 "policy object predicate failed")
(modules-lib-assert-equal
 (marlin-policy-object-id modules-lib-default-route-object)
 "default-route"
 "policy object id mismatch")
(modules-lib-assert-equal
 (marlin-policy-object-family modules-lib-default-route-object)
 "model-route-policy"
 "policy object family mismatch")
(modules-lib-assert-equal
 (.get (marlin-add-object modules-lib-memory-trigger-object) kind)
 marlin-policy-object-operation-kind
 "add operation kind mismatch")
(modules-lib-assert-equal
 (marlin-policy-object-operation?
  (marlin-disable-object "human-review-policy" "human-review"))
 #t
 "disable operation predicate failed")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack kind)
 marlin-policy-pack-kind
 "pack kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack id)
 "modules-lib-prefab-pack"
 "pack id mismatch")
(modules-lib-assert-equal
 (marlin-policy-object-ids (.get modules-lib-policy-pack policy-objects))
 '("fast-route" "human-review" "memory-trigger")
 "post-surgery object id inventory mismatch")
(modules-lib-assert-equal
 (marlin-policy-object-families (.get modules-lib-policy-pack policy-objects))
 '("model-route-policy" "human-review-policy" "memory-trigger-policy")
 "post-surgery object family inventory mismatch")
(modules-lib-assert-equal
 marlin-memory-trigger-policy-family
 "memory-trigger-policy"
 "memory trigger family mismatch")
(modules-lib-assert-equal
 (marlin-policy-object-disabled-ids
  (.get modules-lib-policy-pack policy-objects))
 '("human-review")
 "post-surgery disabled object id mismatch")
(def modules-lib-policy-pack-presentation-now
  (marlinPolicyPackPresentation modules-lib-policy-pack))

(def modules-lib-policy-projection-now
  (marlinPolicyProjection modules-lib-policy-pack))

(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now kind)
 marlin-policy-pack-presentation-kind
 "pack presentation kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now pack-kind)
 marlin-policy-pack-kind
 "pack presentation pack kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now pack-id)
 "modules-lib-prefab-pack"
 "pack presentation id mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now policy-pack-inventory-kind)
 marlin-policy-pack-inventory-kind
 "pack presentation inventory kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now policy-object-count)
 3
 "presentation object count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now default-policy-object-count)
 3
 "presentation default object count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now object-operation-count)
 4
 "presentation operation count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now object-surgery-receipt-count)
 4
 "presentation surgery receipt count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now add-operation-count)
 1
 "presentation add count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now remove-operation-count)
 1
 "presentation remove count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now disable-operation-count)
 1
 "presentation disable count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now replace-operation-count)
 1
 "presentation replace count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now matched-surgery-receipt-count)
 4
 "presentation matched receipt count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now disabled-policy-object-count)
 1
 "presentation disabled object count mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now module-system-presentation-kind)
 marlin-module-system-presentation-kind
 "presentation module-system kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now policy-projection-receipt-kind)
 marlin-policy-projection-kind
 "presentation projection receipt kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-projection-now kind)
 marlin-policy-projection-kind
 "policy projection kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-projection-now native-projection-payload-kind)
 marlin-policy-pack-presentation-kind
 "policy projection native payload kind mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-projection-now budget-receipt-owner)
 "rust"
 "policy projection budget owner mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-projection-now policy-composition-owner)
 "poo-flow.scheme"
 "policy projection composition owner mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now rust-parses-scheme-source)
 #f
 "presentation source parsing boundary mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now rust-handler-manufactured)
 #f
 "presentation handler manufacture boundary mismatch")
