;;; -*- Gerbil -*-
;;; Boundary: Downstream agent owns a subagent policy extension object.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-strategy)

(export user-interface-subagent-profile
        user-interface-subagent-route-policy
        user-interface-subagent-context
        user-interface-subagent-policy-extension)

;;; Boundary: The agent-authored subagent profile is a plain POO object.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-profile
  (.o id: "user-interface-review-subagent"
      agent-class: "customer-user-interface"
      lineage: '("root-agent"
                 "user-interface-agent"
                 "user-interface-review-subagent")
      spawn-command: "codex subagent spawn user-interface-review-subagent"))

;;; Boundary: Agent-authored policy stays a typed policy object.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-route-policy
  (make-marlin-deck-runtime-model-route-policy
   "user-interface-subagent-route"
   "openai"
   "gpt-5.4"
   '("codex user-interface")
   '("user-interface-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Test context proves the extension object can match downstream state.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-context
  (make-marlin-deck-runtime-strategy-context
   "user-interface-session"
   '("root-agent" "user-interface-agent")
   '("workspace-ready")
   '("interface.org")
   "customer-user-interface"))

;;; Boundary: Extension conditions remain Scheme policy objects.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-condition
  (make-marlin-deck-runtime-condition-policy
   "user-interface-session"
   '("root-agent" "user-interface-agent")
   '("workspace-ready")
   '("interface.org")
   "customer-user-interface"))

;;; Boundary: Agent subagent launch policy is regular high-order matching.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "user-interface-subagent-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context agent-class) "customer-user-interface")
          (string=? (.get policy provider) "openai")
          (string=? command "codex user-interface spawn-subagent")
          (string=? agent-scope "user-interface-agent")))))

;;; Boundary: The extension object combines subagent, hook, and policy slots.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-policy-extension
  (make-marlin-deck-runtime-subagent-policy-extension
   "user-interface-subagent-policy-extension"
   user-interface-subagent-profile
   user-interface-subagent-route-policy
   user-interface-subagent-condition
   user-interface-subagent-matcher
   (make-marlin-deck-runtime-register-hook-action
    "runtime-catalog-user-interface-hook"
    "runtime-catalog-user-interface-hook")
   '((owner . "user-interface-worker")
     (surface . "agent-authored-subagent-policy"))))
