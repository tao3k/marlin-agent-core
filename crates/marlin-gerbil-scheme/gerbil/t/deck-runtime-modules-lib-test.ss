;;; -*- Gerbil -*-
;;; Boundary: Module tests the public marlinModules user interface.

(import :clan/poo/object
        :marlin/deck-runtime-modules-lib
        :std/test)

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
(check (.get modules-lib-missing-schema-receipt errors)
       => '("option schema is not declared"))
