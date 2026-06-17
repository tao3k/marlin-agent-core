;;; -*- Gerbil -*-
;;; Boundary: Downstream example owns the user interface module configuration.

(import :clan/poo/object
        :modules/lib
        "agent"
        "hooks"
        "interface"
        "loop-continuation"
        "script"
        "subagent"
        "workspace")

(export user-interface-module-config
        user-interface-module-catalog
        user-interface-module-evaluation
        user-interface-module-system-presentation
        user-interface-module-workflow
        user-interface-policy-pack
        user-interface-policy-pack-catalog
        user-interface-policy-pack-presentation)

;;; Boundary: User interface config is a POO module object, not runtime plumbing.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-module-base-config
  (marlinModules
   UserInterfaceModuleConfig
   (.o id: "user-interface-root-module"
       config:
       (.o surface: "downstream-user-interface"
           entry: "interface-workflow"))))

;;; Boundary: Root config extends imports through native POO slot composition.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-module-config
  (.o (:: @ (list user-interface-module-base-config))
      (imports => marlin-imports-append
               (marlin-imports
                (marlin-import "./workspace.ss" user-interface-workspace-module)
                (marlin-import "./agent.ss" user-interface-agent-module)
                (marlin-import
                 "./loop-continuation.ss"
                 user-interface-loop-continuation-module)
                (marlin-import "./script.ss" user-interface-script-module)
                (marlin-import "./hooks.ss" hook-profile)))
      (extensions => marlin-extensions-append
                  (marlin-extensions
                   user-interface-subagent-policy-extension))))

;;; Boundary: Catalog is the user-facing module-system collection boundary.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-module-catalog)
  (marlinModuleCatalog user-interface-module-config))

;;; Boundary: evalModules is the public evaluation entrypoint for this example.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-module-evaluation)
  (marlinEvalModules
   (user-interface-module-catalog)
   "user-interface-root-module"
   '("runtime-catalog-user-interface-hook")))

;;; Boundary: Presentation receipt is the complete scalar module-system view.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-module-system-presentation)
  (marlinModuleSystemPresentation
   (user-interface-module-catalog)
   "user-interface-root-module"
   '("runtime-catalog-user-interface-hook")))

;;; Boundary: Workflow materializes runtime projections for scripts/extensions.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-module-workflow)
  (marlin-module-workflow
   user-interface-module-config
   '("runtime-catalog-user-interface-hook")))

;;; Boundary: Prefab pack object wraps the exported subagent extension.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-policy-object
  (marlinPolicyObject
   "subagent-policy-extension"
   "user-interface-subagent-policy-extension"
   user-interface-subagent-policy-extension
   '((owner . "user-interface-worker") (surface . "prefab-object"))))

;;; Boundary: Continuation policy stays a POO object before Rust projection.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-continuation-policy-object
  (marlinPolicyObject
   "continuation-profile"
   "user-interface-loop-continuation"
   user-interface-loop-continuation-profile
   '((owner . "user-interface-worker") (surface . "prefab-object"))))

;;; Boundary: Hook selector object only names an existing Rust catalog handler.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-hook-policy-object
  (marlinPolicyObject
   "hook-selection-policy"
   "runtime-catalog-user-interface-hook"
   (.o hook-id: "runtime-catalog-user-interface-hook"
       action: "register")
   '((owner . "user-interface-worker") (surface . "prefab-object"))))

;;; Boundary: Added memory trigger is normal policy furniture.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-memory-policy-object
  (marlinPolicyObject
   "memory-trigger-policy"
   "user-interface-memory-trigger"
   (.o trigger: "context-pressure"
       action: "compact")
   '((owner . "user-interface-worker") (surface . "prefab-object"))))

;;; Boundary: Replacement object keeps typed projection separate from merge rules.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-continuation-projection-object
  (marlinPolicyObject
   "continuation-profile"
   "user-interface-continuation-projection"
   user-interface-loop-continuation-projection
   '((owner . "user-interface-worker") (surface . "prefab-object"))))

;;; Boundary: User-facing prefab pack composes modules plus policy objects.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(defmarlin-policy-pack user-interface-policy-pack
  (id "user-interface-prefab-pack")
  (module user-interface-module-config)
  (policy-objects user-interface-subagent-policy-object
                  user-interface-continuation-policy-object
                  user-interface-hook-policy-object)
  (object-operations
   (marlin-add-object
    user-interface-memory-policy-object
    "add memory trigger furniture")
   (marlin-remove-object
    "hook-selection-policy"
    "runtime-catalog-user-interface-hook"
    "Rust catalog owns hook handlers")
   (marlin-disable-object
    "subagent-policy-extension"
    "user-interface-subagent-policy-extension"
    "disabled by downstream object surgery")
   (marlin-replace-object
    "continuation-profile"
    "user-interface-loop-continuation"
    user-interface-continuation-projection-object
    "replace profile object with typed projection object"))
  (allowed-hook-ids "runtime-catalog-user-interface-hook")
  (metadata '((owner . "user-interface-worker") (surface . "prefab-pack"))))

;;; Boundary: Pack catalog is the prefab collection entrypoint.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-policy-pack-catalog)
  (marlinPackCatalog user-interface-policy-pack))

;;; Boundary: Presentation gives Rust/debug tools a scalar prefab receipt.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-policy-pack-presentation)
  (marlinPolicyPackPresentation user-interface-policy-pack))
