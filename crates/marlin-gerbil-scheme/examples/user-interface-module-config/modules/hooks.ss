;;; -*- Gerbil -*-
;;; Boundary: Downstream example module owns runtime-hook catalog config.

(import :clan/poo/object
        :marlin/deck-runtime-modules-lib)

(export UserInterfaceHookProfile
        hook-profile)

;;; Boundary: Hook profile names existing Rust-owned runtime catalog entries.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def UserInterfaceHookProfile
  (marlin-module-interface
   "UserInterfaceHookProfile"
   (.o hook-id: (marlin-string-constant "runtime-catalog-user-interface-hook")
       hook-action: (marlin-string-constant "register")
       hook-owner: (marlin-string-default "user-interface-worker"))
   '((owner . "user-interface-worker"))))

;;; Boundary: Hook module is an actual reusable user module example.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def hook-profile
  (marlinModules
   UserInterfaceHookProfile
   (.o id: "user-interface-hook-module"
       config:
       (.o hook-id: "runtime-catalog-user-interface-hook"
           hook-action: "register"
           hook-owner: "user-interface-worker"))))
