;;; -*- Gerbil -*-
;;; Boundary: Gerbil package smoke for the Marlin <-> poo-flow policy contract.

(import :clan/poo/object
        :config-interface/modules/prefabs/user-interface-delivery)

(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

(def marline-kernel-workspace
  (UserInterfaceWorkspace
   (.o workspace-root: "marline-kernel-workspace"
       interface-file: "interface.org"
       state-file: "state/marline-kernel-state.org"
       model-profile: "kernel-policy")))

(def marline-kernel-delivery
  (UserInterfaceDeliveryReceipt marline-kernel-workspace))

(check (.get marline-kernel-delivery marlin-loops-policy-owner)
       => "marlin")
(check (.get marline-kernel-delivery
             marlin-loops-policy-control-plane-owner)
       => "poo-flow")
(check (.get marline-kernel-delivery
             marlin-loops-policy-runtime-execution-owner)
       => "marlin-agent-core")
(check (.get marline-kernel-delivery
             marlin-loops-policy-receipt-contract-count)
       => 8)

(display "marlin-poo-flow-scenario-ok")
(newline)
(display "control-plane-owner=")
(display (.get marline-kernel-delivery
               marlin-loops-policy-control-plane-owner))
(newline)
(display "runtime-owner=")
(display (.get marline-kernel-delivery
               marlin-loops-policy-runtime-execution-owner))
(newline)
