;;; -*- Gerbil -*-
;;; Boundary: Downstream example module owns agent-facing config.

(import :clan/poo/object
        :modules/lib)

(export UserInterfaceAgentProfile
        user-interface-agent-module)

;;; Boundary: Agent profile is a user-imported interface.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def UserInterfaceAgentProfile
  (marlin-module-interface
   "UserInterfaceAgentProfile"
   (.o agent-scope: (marlin-string-constant "user-interface-agent")
       agent-class: (marlin-string-constant "customer-user-interface")
       model-profile: (marlin-string-default "interactive"))
   '((owner . "user-interface-worker"))))

;;; Boundary: Agent module is an actual reusable user module example.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-agent-module
  (marlinModules
   UserInterfaceAgentProfile
   (.o id: "user-interface-agent-module"
       config:
       (.o agent-scope: "user-interface-agent"
           agent-class: "customer-user-interface"
           model-profile: "interactive"))))
