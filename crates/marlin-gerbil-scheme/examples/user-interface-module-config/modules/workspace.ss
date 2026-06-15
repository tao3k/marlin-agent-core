;;; -*- Gerbil -*-
;;; Boundary: Downstream example module owns workspace-facing config.

(import :clan/poo/object
        :marlin/deck-runtime-modules-lib)

(export UserInterfaceWorkspaceProfile
        user-interface-workspace-module)

;;; Boundary: Workspace profile is a user-imported interface.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def UserInterfaceWorkspaceProfile
  (marlin-module-interface
   "UserInterfaceWorkspaceProfile"
   (.o workspace-root: (marlin-string-constant "user-interface-module-config")
       interface-file: (marlin-string-constant "interface.org")
       state-file: (marlin-string-constant "state/worker-state.org"))
   '((owner . "user-interface-worker"))))

;;; Boundary: Workspace module is an actual reusable user module example.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-workspace-module
  (marlinModules
   UserInterfaceWorkspaceProfile
   (.o id: "user-interface-workspace-module"
       config:
       (.o workspace-root: "user-interface-module-config"
           interface-file: "interface.org"
           state-file: "state/worker-state.org"))))
