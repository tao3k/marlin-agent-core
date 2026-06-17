;;; -*- Gerbil -*-
;;; Boundary: Hook selection policy objects for prefab packs.

package: modules

(import (only-in :clan/poo/object .o)
        :modules/policy-object)

(export marlin-hook-selection-policy-family
        marlinHookSelectionPolicy
        marlinDefaultHookSelectionPolicy)

;;; Boundary: Hook policy selects catalog ids; Rust owns handler lookup.
;; String
(def marlin-hook-selection-policy-family
  "hook-selection-policy")

;;; Boundary: User-authored .ss files name existing Rust catalog handlers.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinHookSelectionPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "hook-selection-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-hook-selection-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished hook selector registers only a Rust-known handler id.
;; PolicyObject <- Void
(def (marlinDefaultHookSelectionPolicy)
  (marlinHookSelectionPolicy
   "default-hook"
   (.o hook-id: "runtime-catalog-default-hook"
       action: "register")
   '((owner . "marlin") (surface . "default-hook-selection-policy"))))
