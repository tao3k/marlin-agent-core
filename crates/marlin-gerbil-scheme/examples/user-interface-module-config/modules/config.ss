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

(export user-interface-module-config)

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
