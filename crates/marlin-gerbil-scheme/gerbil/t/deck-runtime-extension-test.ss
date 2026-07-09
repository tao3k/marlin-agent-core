;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-strategy
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def extension-test-policy
  (make-marlin-deck-runtime-model-route-policy
   "extension-policy"
   "openai"
   "gpt-5.4"
   '("marlin-extension")
   '("extension-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def extension-test-context
  (make-marlin-deck-runtime-strategy-context
   "session-extension"
   '("root-agent" "extension-agent")
   '("workspace-dirty")
   '("org-memory-extension")
   "customer-extension"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def extension-condition
  (make-marlin-deck-runtime-condition-policy
   "session-extension"
   '("root-agent" "extension-agent")
   '("workspace-dirty")
   '("org-memory-extension")
   "customer-extension"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def extension-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "extension-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context agent-class) "customer-extension")
          (string=? (.get policy provider) "openai")
          (string=? command "marlin extension apply")
          (string=? agent-scope "extension-agent")))))

;;; Boundary: Agent-authored subagent profiles remain POO extension slots.
;; MarlinResult <- MarlinInput
(def extension-subagent-profile
  (.o id: "customer-subagent"
      agent-class: "customer-extension"
      lineage: '("root-agent" "extension-agent" "customer-subagent")
      spawn-command: "marlin subagent spawn customer-subagent"))

;;; Boundary: Subagent policy matching is still an extension object matcher.
;; MarlinResult <- MarlinInput
(def subagent-policy-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "subagent-policy-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context agent-class) "customer-extension")
          (string=? (.get policy provider) "openai")
          (string=? command "marlin subagent apply")
          (string=? agent-scope "extension-agent")))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-extension customer-extension
  "customer-extension"
  '("dynamic-hook-action" "customer-agent-policy" "high-order-matcher")
  extension-condition
  extension-matcher
  (make-marlin-deck-runtime-register-hook-action
   "runtime-catalog-extension-hook"
   "runtime-catalog-extension-hook")
  '((owner . "customer") (safety . "catalog-only")))

;;; Boundary: Extensions may delegate action choice to Scheme selectors.
;; MarlinResult <- MarlinInput
(def extension-hook-selector
  (make-marlin-deck-runtime-dynamic-hook-selector
   "extension-hook-selector"
   (list
    (make-marlin-deck-runtime-dynamic-hook-case
     "extension-register"
     extension-condition
     (marlin-deck-runtime-command-prefix-matcher
      "extension-register-prefix"
      '("marlin extension apply"))
     (make-marlin-deck-runtime-register-hook-action
      "runtime-catalog-extension-hook"
      "runtime-catalog-extension-hook")))
   (make-marlin-deck-runtime-allow-hook-action)))

;;; Boundary: Selector-backed extensions still project typed actions to Rust.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-extension selector-extension
  "selector-extension"
  '("dynamic-hook-action" "customer-agent-policy" "high-order-matcher")
  extension-condition
  extension-matcher
  extension-hook-selector
  '((owner . "customer") (safety . "selector-catalog-only")))

;;; Boundary: Upstream provides extension prototypes; agents fill POO slots.
;; MarlinResult <- MarlinInput
(def subagent-policy-extension
  (make-marlin-deck-runtime-subagent-policy-extension
   "customer-subagent-policy-extension"
   extension-subagent-profile
   extension-test-policy
   extension-condition
   subagent-policy-matcher
   (make-marlin-deck-runtime-register-hook-action
    "runtime-catalog-extension-hook"
    "runtime-catalog-extension-hook")
   '((owner . "customer") (surface . "subagent-policy"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def unsafe-register-extension
  (make-marlin-deck-runtime-extension
   "unsafe-register-extension"
   '("dynamic-hook-action")
   extension-condition
   extension-matcher
   (make-marlin-deck-runtime-register-hook-action
    "not-in-runtime-catalog"
    "not-in-runtime-catalog")
   '()))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def unsafe-capability-extension
  (make-marlin-deck-runtime-extension
   "unsafe-capability-extension"
   '("dynamic-hook-action" "raw-rust-handler")
   extension-condition
   extension-matcher
   (make-marlin-deck-runtime-allow-hook-action)
   '()))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-test-catalog)
  (make-marlin-deck-runtime-extension-catalog
   '("runtime-catalog-extension-hook")
   '()))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-extension-safety)
  (let ((valid-report
         (marlin-deck-runtime-validate-extension
          customer-extension
          '("runtime-catalog-extension-hook")))
        (invalid-register-report
         (marlin-deck-runtime-validate-extension
          unsafe-register-extension
          '("runtime-catalog-extension-hook")))
        (invalid-capability-report
         (marlin-deck-runtime-validate-extension
          unsafe-capability-extension
          '("runtime-catalog-extension-hook"))))
    (check (.get valid-report kind)
           => marlin-deck-runtime-extension-safety-report-kind)
    (check (.get valid-report valid) => #t)
    (check (.get invalid-register-report valid) => #f)
    (check (.get invalid-register-report errors)
           => '("dynamic-hook-catalog-id-not-allowed"))
    (check (.get invalid-capability-report valid) => #f)
    (check (.get invalid-capability-report errors)
           => '("extension-capability-not-allowed"))))

;;; Boundary: Agent-authored subagent policy objects use the normal extension path.
;; MarlinResult <- MarlinInput
(def (check-subagent-policy-extension)
  (let* ((valid-report
          (marlin-deck-runtime-validate-extension
           subagent-policy-extension
           '("runtime-catalog-extension-hook")))
         (catalog
          (marlin-deck-runtime-extension-catalog-add
           (extension-test-catalog)
           subagent-policy-extension))
         (receipt
          (marlin-deck-runtime-extension-policy-receipt
           catalog
           extension-test-context
           extension-test-policy
           "marlin subagent apply"
           "extension-agent"))
         (action (.get receipt dynamic-hook-action)))
    (check (.get valid-report valid) => #t)
    (check (if (member "subagent-policy"
                       (.get subagent-policy-extension capabilities))
             #t
             #f)
           => #t)
    (check (.get (.get subagent-policy-extension subagent-profile) id)
           => "customer-subagent")
    (check (.get (.get subagent-policy-extension policy) name)
           => "extension-policy")
    (check (.get receipt matched) => #t)
    (check (.get receipt extension-id)
           => "customer-subagent-policy-extension")
    (check (.get action action) => "register")
    (check (.get action hook-id) => "runtime-catalog-extension-hook")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-extension-catalog)
  (let ((catalog
         (marlin-deck-runtime-extension-catalog-add
          (extension-test-catalog)
          customer-extension)))
    (check (.get catalog kind)
           => marlin-deck-runtime-extension-catalog-kind)
    (check (.get (marlin-deck-runtime-extension-catalog-find
                  catalog
                  "customer-extension")
                 id)
           => "customer-extension")
    (check (.get (marlin-deck-runtime-extension-catalog-select
                  catalog
                  extension-test-context
                  extension-test-policy
                  "marlin extension apply"
                  "extension-agent")
                 id)
           => "customer-extension")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-extension-receipt)
  (let* ((catalog
          (marlin-deck-runtime-extension-catalog-add
           (extension-test-catalog)
           customer-extension))
         (receipt
          (marlin-deck-runtime-extension-policy-receipt
           catalog
           extension-test-context
           extension-test-policy
           "marlin extension apply"
           "extension-agent"))
         (miss
          (marlin-deck-runtime-extension-policy-receipt
           catalog
           extension-test-context
           extension-test-policy
           "cargo test"
           "extension-agent"))
         (selector-catalog
          (marlin-deck-runtime-extension-catalog-add
           (extension-test-catalog)
           selector-extension))
         (selector-receipt
          (marlin-deck-runtime-extension-policy-receipt
           selector-catalog
           extension-test-context
           extension-test-policy
           "marlin extension apply"
           "extension-agent"))
         (action (.get receipt dynamic-hook-action))
         (selection (.get receipt dynamic-hook-selection))
         (selector-action (.get selector-receipt dynamic-hook-action))
         (selector-selection (.get selector-receipt dynamic-hook-selection)))
    (check (.get receipt kind)
           => marlin-deck-runtime-extension-receipt-kind)
    (check (.get receipt matched) => #t)
    (check (.get receipt extension-id) => "customer-extension")
    (check (.get action action) => "register")
    (check (.get action hook-id) => "runtime-catalog-extension-hook")
    (check (.get selection source) => "extension-action")
    (check (.get selection selector) => #f)
    (check (.get receipt policy-engine) => "scheme-poo-extension")
    (check (.get selector-receipt matched) => #t)
    (check (.get selector-action action) => "register")
    (check (.get selector-selection source) => "selector-case")
    (check (.get selector-selection selector) => "extension-hook-selector")
    (check (.get selector-selection matched-case) => "extension-register")
    (check (.get miss matched) => #f)
    (check (.get miss dynamic-hook-action) => #f)
    (check (.get miss dynamic-hook-selection) => #f)))

(check-extension-safety)
(check-subagent-policy-extension)
(check-extension-catalog)
(check-extension-receipt)
