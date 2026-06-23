;;; -*- Gerbil -*-
;;; Boundary: Policy modules wrap Scheme/POO policy composition.

package: marlin/modules

(import (only-in :clan/poo/object .get .has? .o object?)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-modules
                 poo-flow-imports
                 poo-flow-extensions
                 poo-flow-module-name
                 poo-flow-module-object-ref/default
                 poo-flow-scheme-owner)
        :marlin/modules/kinds)

(export marlinPolicyModule
        defmarlin-policy-module
        marlin-policy-module?)

;;; Boundary: Policy modules keep policy composition in Scheme/POO.
;; MarlinResult <- MarlinInput
(def (marlinPolicyModule interface module-config)
  (let (module-value (poo-flow-modules interface module-config))
    (.o (:: @ (list module-value))
        policy-module-kind: marlin-policy-module-kind
        module-kind: marlin-modules-kind
        id: (poo-flow-module-name module-value)
        policy-family:
        (poo-flow-module-object-ref/default
         module-config
         'policy-family
         "extension-policy")
        projection-target:
        (poo-flow-module-object-ref/default
         module-config
         'projection-target
         "extension-policy-receipt")
        receipt-kind:
        (poo-flow-module-object-ref/default
         module-config
         'receipt-kind
         "marlin-deck-runtime.extension-receipt.v1")
        gate-profile:
        (poo-flow-module-object-ref/default
         module-config
         'gate-profile
         "policy-substrate")
        rust-kernel-owner: "rust"
        scheme-policy-owner: poo-flow-scheme-owner
        replayable: #t)))

;;; Boundary: Level-1 user API expands to the POO policy module object.
;; MarlinResult <- MarlinInput
(defrules defmarlin-policy-module ()
  ((_ binding
      interface
      (id module-id)
      (imports import-value ...)
      (config config-object)
      (extensions extension-value ...)
      (scripts script-value ...)
      (policy-family policy-family-value)
      (projection-target projection-target-value)
      (receipt-kind receipt-kind-value)
      (gate-profile gate-profile-value)
      (metadata metadata-value))
   (def binding
     (marlinPolicyModule
      interface
      (.o id: module-id
          imports: (poo-flow-imports import-value ...)
          config: config-object
          extensions: (poo-flow-extensions extension-value ...)
          scripts: (list script-value ...)
          policy-family: policy-family-value
          projection-target: projection-target-value
          receipt-kind: receipt-kind-value
          gate-profile: gate-profile-value
          metadata: metadata-value)))))

;;; Boundary: Policy module detection is typed, not based on source syntax.
;; MarlinResult <- MarlinInput
(def (marlin-policy-module? value)
  (and (object? value)
       (.has? value policy-module-kind)
       (string=? (.get value policy-module-kind) marlin-policy-module-kind)))
