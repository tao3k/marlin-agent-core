;;; -*- Gerbil -*-
;;; Boundary: Downstream example owns only user-facing workspace choices.

(import :clan/poo/object
        :modules/prefabs/user-interface-delivery)

(export user-interface-module-config
        user-interface-delivery-receipt
        user-interface-policy-apply
        user-interface-policy-projection)

;;; Boundary: User layer moves into the furnished upstream prefab.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-module-config
  (UserInterfaceWorkspace
   (.o workspace-root: "user-interface-module-config"
       interface-file: "interface.org"
       state-file: "state/worker-state.org"
       model-profile: "interactive")))

;;; Boundary: Delivery receipt is the user-facing handoff to Rust/debug tools.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-delivery-receipt)
  (UserInterfaceDeliveryReceipt user-interface-module-config))

;;; Boundary: Policy apply is the user-facing delivery action.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-policy-apply)
  (UserInterfaceApply user-interface-module-config))

;;; Boundary: Policy projection is the fixed Rust-facing protocol envelope.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-policy-projection)
  (UserInterfacePolicyProjection user-interface-module-config))
