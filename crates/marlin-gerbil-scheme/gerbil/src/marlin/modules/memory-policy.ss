;;; -*- Gerbil -*-
;;; Boundary: Modular memory policy objects for prefab packs.

package: marlin/modules

(import (only-in :clan/poo/object .o)
        :marlin/modules/policy-object)

(export marlin-memory-recall-policy-family
        marlin-memory-trigger-policy-family
        marlin-memory-retention-policy-family
        marlin-memory-visibility-policy-family
        marlinMemoryRecallPolicy
        marlinMemoryTriggerPolicy
        marlinMemoryRetentionPolicy
        marlinMemoryVisibilityPolicy
        marlinDefaultMemoryRecallPolicy
        marlinDefaultMemoryTriggerPolicy
        marlinDefaultMemoryRetentionPolicy
        marlinDefaultMemoryVisibilityPolicy)

;;; Boundary: Memory is a Scheme-owned policy family, not Rust merge logic.
;; String
(def marlin-memory-recall-policy-family
  "memory-recall-policy")

;; String
(def marlin-memory-trigger-policy-family
  "memory-trigger-policy")

(def marlin-memory-retention-policy-family
  "memory-retention-policy")

;; String
(def marlin-memory-visibility-policy-family
  "memory-visibility-policy")

;;; Boundary: Recall policy selects memory surfaces; Rust owns graph reads.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinMemoryRecallPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "memory-recall-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-memory-recall-policy-family
     object-id-value
     payload-value
     metadata-value)))

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

;;; Boundary: Retention policy composes storage intent; Rust owns snapshots.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinMemoryRetentionPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "memory-retention-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-memory-retention-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Visibility policy declares scope; Rust validates session access.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinMemoryVisibilityPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "memory-visibility-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-memory-visibility-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Default recall uses typed Org memory references, not raw bodies.
;; PolicyObject <- Void
(def (marlinDefaultMemoryRecallPolicy)
  (marlinMemoryRecallPolicy
   "default-memory-recall"
   (.o query-scope: "project-session"
       source: "org-memory"
       projection-target: "project-memory-recall-request")
   '((owner . "marlin") (surface . "default-memory-policy"))))

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

;;; Boundary: Default retention is policy furniture; Rust owns persistence.
;; PolicyObject <- Void
(def (marlinDefaultMemoryRetentionPolicy)
  (marlinMemoryRetentionPolicy
   "default-memory-retention"
   (.o retention: "session-local"
       snapshot: "summary-only"
       storage-owner: "rust-org-memory")
   '((owner . "marlin") (surface . "default-memory-policy"))))

;;; Boundary: Default visibility contracts child memory access through Rust.
;; PolicyObject <- Void
(def (marlinDefaultMemoryVisibilityPolicy)
  (marlinMemoryVisibilityPolicy
   "default-memory-visibility"
   (.o visibility: "parent-granted"
       child-session-default: "deny"
       projection-target: "memory-visibility-receipt")
   '((owner . "marlin") (surface . "default-memory-policy"))))
