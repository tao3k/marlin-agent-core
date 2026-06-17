;;; -*- Gerbil -*-
;;; Boundary: Downstream example interface owns the typed user config surface.

(import :clan/poo/object
        :marlin/modules/lib)

(export UserInterfaceModuleConfig)

;;; Boundary: Interface mirrors Nickel-style imported type annotations.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def UserInterfaceModuleConfig
  (marlin-module-interface
   "UserInterfaceModuleConfig"
   (.o surface: (marlin-string-constant "downstream-user-interface")
       entry: (marlin-string-constant "interface-workflow")
       layer: (marlin-string-optional))
   '((owner . "user-interface-worker"))))
