;;; -*- Gerbil -*-
;;; Boundary: Policy modules wrap Scheme/POO policy composition.

package: modules

(import (only-in :clan/poo/object .get .has? .o object?)
        :modules/kinds
        :modules/core)

(export marlinPolicyModule
        defmarlin-policy-module
        marlin-policy-module?)

;;; Boundary: Policy modules keep policy composition in Scheme/POO.
;; MarlinResult <- MarlinInput
(def (marlinPolicyModule interface module-config)
  (let (module-value (marlinModules interface module-config))
    (.o (:: @ (list module-value))
        kind: marlin-policy-module-kind
        module-kind: marlin-modules-kind
        id: (.get module-value id)
        policy-family:
        (marlin-module-object-ref/default
         module-config
         'policy-family
         "extension-policy")
        projection-target:
        (marlin-module-object-ref/default
         module-config
         'projection-target
         "extension-policy-receipt")
        receipt-kind:
        (marlin-module-object-ref/default
         module-config
         'receipt-kind
         "marlin-deck-runtime.extension-receipt.v1")
        gate-profile:
        (marlin-module-object-ref/default
         module-config
         'gate-profile
         "policy-substrate")
        rust-kernel-owner: "rust"
        scheme-policy-owner: "gerbil-poo"
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
          imports: (marlin-imports import-value ...)
          config: config-object
          extensions: (marlin-extensions extension-value ...)
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
       (.has? value kind)
       (string=? (.get value kind) marlin-policy-module-kind)))
