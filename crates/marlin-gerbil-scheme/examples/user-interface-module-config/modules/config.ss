;;; -*- Gerbil -*-
;;; Boundary: Downstream example owns the user interface module configuration.

(import :clan/poo/object
        :marlin/deck-runtime-modules-lib
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
        user-interface-module-workflow)

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
