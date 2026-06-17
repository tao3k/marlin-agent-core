;;; -*- Gerbil -*-
;;; Boundary: Downstream example owns a POO graph-loop continuation profile.

(import :clan/poo/object
        :marlin/deck-runtime-loop-graph
        :marlin/modules/lib
        :marlin/graph-loop-continuation-native-projection)

(export UserInterfaceLoopContinuationProfile
        user-interface-loop-continuation-module
        user-interface-loop-continuation-base-profile
        user-interface-loop-continuation-profile
        user-interface-loop-continuation-projection)

;;; Boundary: User module interface stays concise; profile details are POO data.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def UserInterfaceLoopContinuationProfile
  (marlin-module-interface
   "UserInterfaceLoopContinuationProfile"
   (.o continuation-profile:
       (marlin-string-constant "user-interface-loop-continuation"))
   '((owner . "user-interface-worker"))))

;;; Boundary: Continuation node names a Rust-owned executor catalog handle.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-continuation-node
  (make-marlin-deck-runtime-loop-node
   "policy"
   "gerbil.poo.policy"
   '(("source" . "poo")
     ("workspace" . "user-interface-module-config"))))

;;; Boundary: Downstream graph declaration is a regular POO value.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(defmarlin-deck-runtime-loop-graph user-interface-loop-continuation-graph
  "user-interface-continuation-graph"
  (user-interface-loop-continuation-node)
  ())

;;; Boundary: Scheme compiles graph shape; Rust validates before execution.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-continuation-compiled-graph
  (marlin-deck-runtime-compile-loop-graph
   user-interface-loop-continuation-graph))

;;; Boundary: Base profile is intentionally terminal and safe by default.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(defmarlin-graph-loop-continuation-profile
  user-interface-loop-continuation-base-profile
  "user-interface-loop-continuation"
  (make-marlin-graph-loop-continuation-stop-completed-action)
  '("poo_continuation=default_stop"))

;;; Boundary: POO extension overrides only action and diagnostics lazily.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-continuation-profile
  (.o (:: @ (list user-interface-loop-continuation-base-profile))
      action:
      (make-marlin-graph-loop-continuation-continue-with-graph-action
       user-interface-loop-continuation-compiled-graph)
      diagnostics:
      '("poo_continuation=continue"
        "workspace=user-interface-module-config")))

;;; Boundary: Projection is a typed POO object handed to the Rust native API.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-continuation-projection
  (marlin-graph-loop-continuation-next-action
   user-interface-loop-continuation-profile))

;;; Boundary: Module config imports the reusable continuation profile file.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-continuation-module
  (marlinModules
   UserInterfaceLoopContinuationProfile
   (.o id: "user-interface-loop-continuation-module"
       config:
       (.o continuation-profile: "user-interface-loop-continuation"))))
