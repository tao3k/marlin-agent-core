;;; -*- Gerbil -*-
;;; Boundary: Failure recovery policy objects for prefab packs.

package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-object)

(export marlin-failure-recovery-policy-family
        marlinFailureRecoveryPolicy
        marlinDefaultFailureRecoveryPolicy)

;;; Boundary: Failure policy is Scheme strategy, not Rust retry plumbing.
;; String
(def marlin-failure-recovery-policy-family
  "failure-recovery-policy")

;;; Boundary: User-authored .ss files extend recovery behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinFailureRecoveryPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "failure-recovery-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-failure-recovery-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished recovery declares policy; Rust owns execution budget.
;; PolicyObject <- Void
(def (marlinDefaultFailureRecoveryPolicy)
  (marlinFailureRecoveryPolicy
   "default-failure-recovery"
   (.o retry-budget: "bounded"
       recovery: "receipt-driven"
       query-family: "failure-query-family"
       budget-owner: "rust")
   '((owner . "marlin") (surface . "default-failure-policy"))))
