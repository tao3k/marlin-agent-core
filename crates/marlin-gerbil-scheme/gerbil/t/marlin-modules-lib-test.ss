;;; -*- Gerbil -*-
;;; Boundary: Module tests the public marlinModules user interface.

(import :clan/poo/object
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
  (marlin-module-interface
   "ModulesLibBase"
   (.o layer: (marlin-string-constant "base"))
   '((owner . "modules-lib-test"))))

;;; Boundary: User config references an imported interface instead of schemas.
;; MarlinResult <- MarlinInput
(def modules-lib-base
  (marlinModules
   modules-lib-base-interface
   (.o id: "modules-lib-base"
       config:
       (.o layer: "base"))))

;;; Boundary: Root interface is separate from the user config record.
;; MarlinResult <- MarlinInput
(def modules-lib-root-interface
  (marlin-module-interface
   "ModulesLibRoot"
   (.o surface: (marlin-string-constant "example")
       entry: (marlin-string-constant "workflow"))
   '((owner . "modules-lib-test"))))

;;; Boundary: Root config imports another marlinModules object directly.
;; MarlinResult <- MarlinInput
(def modules-lib-root-base
  (marlinModules
   modules-lib-root-interface
   (.o id: "modules-lib-root"
       config:
       (.o surface: "example"
           entry: "workflow"))))

;;; Boundary: Root config extends imports through native POO slot composition.
;; MarlinResult <- MarlinInput
(def modules-lib-root
  (.o (:: @ (list modules-lib-root-base))
      (imports => marlin-imports-append
               (marlin-imports
                (marlin-import "./base.ss" modules-lib-base)
                (marlin-import modules-lib-base)))))

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

;;; Boundary: Evaluation proves imported marlinModules are applied recursively.
;; MarlinResult <- MarlinInput
(def modules-lib-evaluation
  (.get modules-lib-workflow evaluation))

;;; Boundary: Missing schemas return typed receipts instead of runtime failures.
;; MarlinResult <- MarlinInput
(def modules-lib-empty-interface
  (marlin-module-interface
   "ModulesLibEmpty"
   (.o)
   '((owner . "modules-lib-test"))))

;;; Boundary: Missing schemas return typed receipts instead of runtime failures.
;; MarlinResult <- MarlinInput
(def modules-lib-missing-schema
  (marlinModules
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
  (marlinPolicyObject
   "memory-trigger-policy"
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

;;; Boundary: Presentation is the scalar proof Rust/debug tooling consumes.
;; MarlinResult <- MarlinInput
(def modules-lib-policy-pack-presentation
  (marlinPolicyPackPresentation modules-lib-policy-pack))

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
(check (.get modules-lib-eval-result validation-receipt-count) => 4)
(check (.get modules-lib-eval-result policy-extension-object-count) => 0)
(check (.get modules-lib-presentation kind)
       => marlin-module-system-presentation-kind)
(check (.get modules-lib-presentation projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get modules-lib-presentation import-graph-owner)
       => "gerbil-module-system")
(check (.get modules-lib-presentation option-merge-owner)
       => "gerbil-poo")
(check (.get modules-lib-presentation native-projection-payload-owner)
       => "rust")
(check (.get modules-lib-presentation rust-parses-scheme-source)
       => #f)
(check (.get modules-lib-presentation scheme-manufactures-rust-handlers)
       => #f)
(check (.get (car (.get modules-lib-root imports)) kind)
       => marlin-module-import-kind)
(check (.get (.get (car (.get modules-lib-root imports)) source-ref) kind)
       => marlin-import-source-ref-kind)
(check (.get (.get (.get (car (.get modules-lib-root imports)) source-ref) source)
             kind)
       => marlin-import-local-source-kind)
(check (.get (.get (.get (car (.get modules-lib-root imports)) source-ref) source)
             path)
       => "./base.ss")
(check (.get (cadr (.get modules-lib-root imports)) source-ref) => #f)
(check (.get (.get (cadr (.get modules-lib-root imports)) profile) id)
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
       => '("modules-lib-base" "modules-lib-root"))
(check (map (lambda (option) (.get option id))
            (.get modules-lib-evaluation options))
       => '("surface" "entry" "layer"))
(check (.get modules-lib-missing-schema-receipt valid?) => #f)
(unless (= (length (.get modules-lib-missing-schema-receipt errors)) 1)
  (error "expected one missing schema error"))
(unless (string=? (car (.get modules-lib-missing-schema-receipt errors))
                  "option schema is not declared")
  (error "unexpected missing schema error"))

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
(def modules-lib-policy-pack-presentation-now
  (marlinPolicyPackPresentation modules-lib-policy-pack))

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
 (.get modules-lib-policy-pack-presentation-now policy-object-count)
 3
 "presentation object count mismatch")
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
 (.get modules-lib-policy-pack-presentation-now rust-parses-scheme-source)
 #f
 "presentation source parsing boundary mismatch")
(modules-lib-assert-equal
 (.get modules-lib-policy-pack-presentation-now rust-handler-manufactured)
 #f
 "presentation handler manufacture boundary mismatch")
