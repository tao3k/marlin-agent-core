;;; -*- Gerbil -*-
;;; Boundary: Evidence graph policy objects for prefab packs.

package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-object)

(export marlin-evidence-graph-policy-family
        marlinEvidenceGraphPolicy
        marlinDefaultEvidenceGraphPolicy)

;;; Boundary: Evidence policy is Scheme composition over Rust-owned evidence.
;; String
(def marlin-evidence-graph-policy-family
  "evidence-graph-policy")

;;; Boundary: User-authored .ss files extend evidence behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinEvidenceGraphPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "evidence-graph-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-evidence-graph-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished evidence policy declares intent; Rust owns receipts.
;; PolicyObject <- Void
(def (marlinDefaultEvidenceGraphPolicy)
  (marlinEvidenceGraphPolicy
   "default-evidence-graph"
   (.o query-family: "evidence-query-family"
       graph-intent: "receipt-linked-evidence"
       evidence-owner: "rust"
       projection-target: "agent-evidence-graph-receipt")
   '((owner . "marlin") (surface . "default-evidence-policy"))))
