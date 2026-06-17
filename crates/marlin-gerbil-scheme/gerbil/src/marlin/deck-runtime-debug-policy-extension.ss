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
        :modules/lib
        :marlin/deck-runtime-strategy-context)

(export marlin-deck-runtime-debug-policy-extension-source
        marlin-deck-runtime-debug-policy-extension
        marlin-deck-runtime-debug-policy-module
        marlin-deck-runtime-debug-policy-pack
        marlin-deck-runtime-debug-policy-pack-catalog
        marlin-deck-runtime-debug-policy-pack-presentation
        marlin-deck-runtime-debug-policy-module-catalog
        marlin-deck-runtime-debug-policy-module-evaluation
        marlin-deck-runtime-debug-policy-module-system-presentation
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

;;; Boundary: The extension itself is a prefab policy object.
;; MarlinResult <- MarlinInput
(def debug-policy-extension-object
  (marlinPolicyObject
   "subagent-policy-extension"
   "debug-policy-extension"
   marlin-deck-runtime-debug-policy-extension
   '((owner . "debug-cli") (surface . "prefab-object"))))

;;; Boundary: A debug review policy shows disabled furniture in receipts.
;; MarlinResult <- MarlinInput
(def debug-policy-review-object
  (marlinPolicyObject
   "human-review-policy"
   "debug-human-review"
   (.o reviewer: "root-agent"
       trigger: "dangerous-tool")
   '((owner . "debug-cli") (surface . "prefab-object"))))

;;; Boundary: A debug hook selector object names an existing Rust handler id.
;; MarlinResult <- MarlinInput
(def debug-policy-hook-object
  (marlinPolicyObject
   "hook-selection-policy"
   "debug-runtime-catalog-hook"
   (.o hook-id: "debug-runtime-catalog-hook"
       action: "register")
   '((owner . "debug-cli") (surface . "prefab-object"))))

;;; Boundary: Replacement model route remains Scheme policy data only.
;; MarlinResult <- MarlinInput
(def debug-policy-fast-route-object
  (marlinPolicyObject
   "model-route-policy"
   "debug-fast-route"
   (.o provider: "openai"
       model: "gpt-5.4-mini"
       route: "debug-fast")
   '((owner . "debug-cli") (surface . "prefab-object"))))

;;; Boundary: Added memory object demonstrates extension by object surgery.
;; MarlinResult <- MarlinInput
(def debug-policy-memory-object
  (marlinPolicyObject
   "memory-trigger-policy"
   "debug-memory-trigger"
   (.o trigger: "context-pressure"
       action: "compact")
   '((owner . "debug-cli") (surface . "prefab-object"))))

;;; Boundary: Debug pack is the prefab/policy-pack surface for the CLI.
;;; Engineering note: object surgery models prefab customization:
;;; Scheme composes add/remove/disable/replace policy objects, while Rust only
;;; validates the projected receipts and resolves existing catalog handlers.
;; MarlinResult <- MarlinInput
(defmarlin-policy-pack marlin-deck-runtime-debug-policy-pack
  (id "debug-policy-prefab-pack")
  (module marlin-deck-runtime-debug-policy-module)
  (policy-objects debug-policy-extension-object
                  debug-policy-review-object
                  debug-policy-hook-object)
  (object-operations
   (marlin-add-object
    debug-policy-memory-object
    "add debug memory policy object")
   (marlin-remove-object
    "hook-selection-policy"
    "debug-runtime-catalog-hook"
    "Rust catalog owns hook handler lookup")
   (marlin-disable-object
    "human-review-policy"
    "debug-human-review"
    "disabled by debug prefab surgery")
   (marlin-replace-object
    "subagent-policy-extension"
    "debug-policy-extension"
    debug-policy-fast-route-object
    "replace extension object with route policy data in pack only"))
  (allowed-hook-ids "debug-runtime-catalog-hook")
  (metadata '((owner . "debug-cli") (surface . "policy-prefab-pack"))))

;;; Boundary: Debug pack catalogs keep pack selection first-class.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-pack-catalog)
  (marlinPackCatalog marlin-deck-runtime-debug-policy-pack))

;;; Boundary: Debug CLI reads pack presentation as scalar facts.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-pack-presentation)
  (marlinPolicyPackPresentation marlin-deck-runtime-debug-policy-pack))

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

;;; Boundary: Full module-system presentation stays Scheme-owned.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-module-system-presentation)
  (marlinModuleSystemPresentation
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
;;; Optimization boundary: reuse one catalog so timing tracks policy selection
;;; and typed receipt projection rather than module workflow construction.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-debug-policy-extension-receipt-loop iterations)
  (let ((catalog (marlin-deck-runtime-debug-policy-extension-catalog)))
    (foldl (lambda (_ receipt)
             (marlin-deck-runtime-debug-policy-extension-receipt-from-catalog
              catalog))
           #f
           (list-tabulate iterations identity))))
