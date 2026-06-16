;;; -*- Gerbil -*-
;;; Boundary: Debug policy extension is a user-authored POO extension object.
;;; The Gerbil module system loads this .ss file; the exported POO object drives policy.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-extension-catalog
        :marlin/deck-runtime-extension-receipt
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-modules-lib
        :marlin/deck-runtime-strategy-context)

(export marlin-deck-runtime-debug-policy-extension-source
        marlin-deck-runtime-debug-policy-extension
        marlin-deck-runtime-debug-policy-module
        marlin-deck-runtime-debug-policy-module-catalog
        marlin-deck-runtime-debug-policy-module-evaluation
        marlin-deck-runtime-debug-policy-module-workflow
        marlin-deck-runtime-debug-policy-extension-catalog
        marlin-deck-runtime-debug-policy-extension-receipt
        marlin-deck-runtime-debug-policy-extension-receipt-loop)

;;; Boundary: This identifies the .ss source managed by the Gerbil module system.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-debug-policy-extension-source
  ":marlin/deck-runtime-debug-policy-extension")

;;; Boundary: Runtime policy input stays typed and reusable across receipts.
;; MarlinResult <- MarlinInput
(def debug-policy-extension-policy
  (make-marlin-deck-runtime-model-route-policy
   "debug-policy"
   "openai"
   "gpt-5.4"
   '("codex")
   '("extension-agent")
   "forked-context"
   "workspace-isolated"))

;;; Boundary: Context matching remains Scheme-owned policy composition.
;; MarlinResult <- MarlinInput
(def debug-policy-extension-context
  (make-marlin-deck-runtime-strategy-context
   "debug-session"
   '("root-agent" "extension-agent")
   '("workspace-clean")
   '("org-memory-debug")
   "customer-extension"))

;;; Boundary: POO extension slots carry agent-facing profile data.
;; MarlinResult <- MarlinInput
(def debug-policy-extension-subagent-profile
  (.o id: "debug-extension-subagent"
      agent-class: "customer-extension"
      lineage: '("root-agent" "extension-agent" "debug-extension-subagent")
      spawn-command: "codex subagent spawn debug-extension-subagent"))

;;; Boundary: Extension condition is composed in Scheme, not Rust.
;; MarlinResult <- MarlinInput
(def debug-policy-extension-condition
  (make-marlin-deck-runtime-condition-policy
   "debug-session"
   '("root-agent" "extension-agent")
   '("workspace-clean")
   '("org-memory-debug")
   "customer-extension"))

;;; Boundary: High-order matcher is the Scheme extension point.
;; MarlinResult <- MarlinInput
(def debug-policy-extension-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "debug-policy-extension-matcher"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context agent-class) "customer-extension")
          (string=? (.get policy provider) "openai")
          (string=? command "codex extension apply")
          (string=? agent-scope "extension-agent")))))

;;; Boundary: This .ss exports the user-authored POO extension object.
;; MarlinResult <- MarlinInput
(defmarlin-policy-extension marlin-deck-runtime-debug-policy-extension
  (source marlin-deck-runtime-debug-policy-extension-source)
  (object
   (make-marlin-deck-runtime-subagent-policy-extension
    "debug-policy-extension"
    debug-policy-extension-subagent-profile
    debug-policy-extension-policy
    debug-policy-extension-condition
    debug-policy-extension-matcher
    (make-marlin-deck-runtime-register-hook-action
     "debug-runtime-catalog-hook"
     "debug-runtime-catalog-hook")
    '((owner . "debug-cli") (surface . "poo-extension-object"))))
  (metadata '((owner . "debug-cli") (surface . "policy-extension-object"))))

;;; Boundary: Policy module is the user-facing module object around extensions.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-debug-policy-module-interface
  (marlin-module-interface
   "DebugPolicyExtensionModule"
   (.o extension-surface: (marlin-string-constant "poo-extension-object")
       projection-target: (marlin-string-constant "extension-policy-receipt"))
   '((owner . "debug-cli") (surface . "policy-substrate-gate"))))

;;; Boundary: Level-1 user API owns imports/config/extensions composition.
;; MarlinResult <- MarlinInput
(defmarlin-policy-module marlin-deck-runtime-debug-policy-module
  marlin-deck-runtime-debug-policy-module-interface
  (id "debug-policy-extension-module")
  (imports)
  (config (.o extension-surface: "poo-extension-object"
              projection-target: "extension-policy-receipt"))
  (extensions marlin-deck-runtime-debug-policy-extension)
  (scripts)
  (policy-family "subagent-policy-extension")
  (projection-target "extension-policy-receipt")
  (receipt-kind marlin-deck-runtime-extension-receipt-kind)
  (gate-profile "policy-substrate")
  (metadata '((owner . "debug-cli") (surface . "policy-substrate-gate"))))

;;; Boundary: Module catalog is a first-class Scheme value.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-module-catalog)
  (marlinModuleCatalog marlin-deck-runtime-debug-policy-module))

;;; Boundary: evalModules is the user-facing evaluation entrypoint.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-module-evaluation)
  (marlinEvalModules
   (marlin-deck-runtime-debug-policy-module-catalog)
   "debug-policy-extension-module"
   '("debug-runtime-catalog-hook")))

;;; Boundary: Policy workflow adds the substrate gate around module evaluation.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-module-workflow)
  (marlin-policy-module-workflow
   marlin-deck-runtime-debug-policy-module
   '("debug-runtime-catalog-hook")))

;;; Boundary: Module management registers extension objects into a Scheme catalog.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-extension-catalog)
  (.get (marlin-deck-runtime-debug-policy-module-workflow)
        extension-catalog))

;;; Boundary: Debug CLI runs the extension policy receipt through typed projection.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-extension-receipt)
  (marlin-deck-runtime-debug-policy-extension-receipt-from-catalog
   (marlin-deck-runtime-debug-policy-extension-catalog)))

;;; Boundary: A caller may reuse a catalog across high-frequency receipt loops.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-extension-receipt-from-catalog catalog)
  (marlin-deck-runtime-extension-policy-receipt
   catalog
   debug-policy-extension-context
   debug-policy-extension-policy
   "codex extension apply"
   "extension-agent"))

;;; Boundary: Performance smoke loops extension policy evaluation in one gxi process.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-extension-receipt-loop iterations)
  (let ((catalog (marlin-deck-runtime-debug-policy-extension-catalog)))
    (let loop ((remaining iterations)
               (receipt #f))
      (if (<= remaining 0)
        receipt
        (loop (- remaining 1)
              (marlin-deck-runtime-debug-policy-extension-receipt-from-catalog
               catalog))))))
