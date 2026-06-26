;;; -*- Gerbil -*-
;;; Boundary: Human review policy objects for prefab packs.

package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-object)

(export marlin-human-review-policy-family
        marlinHumanReviewPolicy
        marlinDefaultHumanReviewPolicy)

;;; Boundary: Human review policy is Scheme strategy over Rust receipts.
;; String
(def marlin-human-review-policy-family
  "human-review-policy")

;;; Boundary: User-authored .ss files extend review behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinHumanReviewPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "human-review-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-human-review-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished review policy declares triggers, not runtime pauses.
;; PolicyObject <- Void
(def (marlinDefaultHumanReviewPolicy)
  (marlinHumanReviewPolicy
   "default-human-review"
   (.o trigger: "high-risk-tool"
       reviewer: "root-agent")
   '((owner . "marlin") (surface . "default-human-review-policy"))))
