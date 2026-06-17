;;; -*- Gerbil -*-
;;; Boundary: Session policy objects for prefab packs.

package: modules

(import (only-in :clan/poo/object .o)
        :modules/policy-object)

(export marlin-session-policy-family
        marlinSessionPolicy
        marlinDefaultSessionPolicy)

;;; Boundary: Session policy is Scheme composition over Rust session receipts.
;; String
(def marlin-session-policy-family
  "session-policy")

;;; Boundary: User-authored .ss files extend session behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinSessionPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "session-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-session-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished session policy declares isolation without runtime code.
;; PolicyObject <- Void
(def (marlinDefaultSessionPolicy)
  (marlinSessionPolicy
   "default-session"
   (.o sharing: "shared"
       isolation: "branch-isolated"
       snapshot: "enabled")
   '((owner . "marlin") (surface . "default-session-policy"))))
