;;; -*- Gerbil -*-
;;; Boundary: Model route policy objects for prefab packs.

package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-object)

(export marlin-model-route-policy-family
        marlinModelRoutePolicy
        marlinDefaultModelRoutePolicy)

;;; Boundary: Model route policy is Scheme strategy over typed Rust routing.
;; String
(def marlin-model-route-policy-family
  "model-route-policy")

;;; Boundary: User-authored .ss files extend model routing as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinModelRoutePolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "model-route-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-model-route-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished route is policy intent; Rust validates model routing.
;; PolicyObject <- Void
(def (marlinDefaultModelRoutePolicy)
  (marlinModelRoutePolicy
   "default-model-route"
   (.o provider: "openai"
       model: "gpt-5.4"
       route: "interactive")
   '((owner . "marlin") (surface . "default-model-route-policy"))))
