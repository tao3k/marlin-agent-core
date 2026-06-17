;;; -*- Gerbil -*-
;;; Boundary: Modular memory policy objects for prefab packs.

package: modules

(import (only-in :clan/poo/object .o)
        :modules/policy-object)

(export marlin-memory-trigger-policy-family
        marlinMemoryTriggerPolicy
        marlinDefaultMemoryTriggerPolicy)

;;; Boundary: Memory is a Scheme-owned policy family, not Rust merge logic.
;; String
(def marlin-memory-trigger-policy-family
  "memory-trigger-policy")

;;; Boundary: User-authored .ss files extend memory behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinMemoryTriggerPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "memory-trigger-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-memory-trigger-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: The furnished house ships a memory trigger, but Rust owns storage.
;; PolicyObject <- Void
(def (marlinDefaultMemoryTriggerPolicy)
  (marlinMemoryTriggerPolicy
   "default-memory-trigger"
   (.o trigger: "context-pressure"
       action: "compact"
       memory-owner: "rust-org-memory"
       projection-target: "org-memory-trigger-receipt")
   '((owner . "marlin") (surface . "default-memory-policy"))))
