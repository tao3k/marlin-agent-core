;;; -*- Gerbil -*-
;;; Boundary: Continuation profile policy objects for prefab packs.

package: modules

(import (only-in :clan/poo/object .o)
        :modules/policy-object)

(export marlin-continuation-profile-policy-family
        marlin-continuation-profile-family
        marlinContinuationProfilePolicy
        marlinContinuationProfile
        marlinDefaultContinuationProfilePolicy)

;;; Boundary: High-level continuation policy declares loop intent.
;; String
(def marlin-continuation-profile-policy-family
  "continuation-profile-policy")

;;; Boundary: Native continuation profiles stay separate from merge plumbing.
;; String
(def marlin-continuation-profile-family
  "continuation-profile")

;;; Boundary: User-authored .ss files extend continuation policy as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinContinuationProfilePolicy
      object-id-value
      payload-value
      . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "continuation-profile-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-continuation-profile-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: User-authored native profiles cross later as typed projection.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinContinuationProfile object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "continuation-profile"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-continuation-profile-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished continuation declares intent; Rust executes graphs.
;; PolicyObject <- Void
(def (marlinDefaultContinuationProfilePolicy)
  (marlinContinuationProfilePolicy
   "default-continuation"
   (.o profile: "balanced"
       graph-intent: "loop-graph")
   '((owner . "marlin") (surface . "default-continuation-policy"))))
