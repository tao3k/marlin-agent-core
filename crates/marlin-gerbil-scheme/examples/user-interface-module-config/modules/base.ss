;;; -*- Gerbil -*-
;;; Boundary: Downstream example base module owns user interface extension objects.

(import :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension-template
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option)

(export user-interface-base-module
        user-interface-extension)

;;; Boundary: User interface policy state stays in Gerbil POO.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-condition
  (make-marlin-deck-runtime-condition-policy
   "user-interface-session"
   (list "root-agent" "user-interface-agent")
   (list "user-interface-worker-ready" "interface-state-open")
   (list "ui-memory" "worker-state-active")
   "customer-user-interface"))

;;; Boundary: User matcher remains an extension-plane decision.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "user-interface-command"
   (lambda (_context _policy command agent-scope)
     (and (string=? command "codex user-interface workflow apply")
          (string=? agent-scope "user-interface-agent")))))

;;; Boundary: Runtime catalog action is named; Scheme does not manufacture Rust handlers.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(defmarlin-deck-runtime-extension user-interface-extension
  "user-interface-worker-extension"
  (list "dynamic-hook-action" "customer-agent-policy" "high-order-matcher")
  user-interface-condition
  user-interface-matcher
  (make-marlin-deck-runtime-register-hook-action
   "runtime-catalog-user-interface-hook"
   "runtime-catalog-user-interface-hook")
  '((owner . "user-interface-worker") (surface . "downstream-user-interface")))

;;; Boundary: Base module exports extension state and base user options.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(defmarlin-deck-runtime-user-module user-interface-base-module
  "user-interface-base-module"
  '()
  (list user-interface-extension)
  '()
  (list
   (make-marlin-deck-runtime-option-config
    "layer"
    "base"
    "user-interface-base-module"
    '((owner . "user-interface-worker")))
   (make-marlin-deck-runtime-option-config
    "surface"
    "downstream-user-interface"
    "user-interface-base-module"
    '((owner . "user-interface-worker"))))
  '((owner . "user-interface-worker")))
