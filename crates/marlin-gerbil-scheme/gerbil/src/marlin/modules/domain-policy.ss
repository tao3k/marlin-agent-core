;;; -*- Gerbil -*-
;;; Boundary: Domain policy objects for furnished prefab packs.

package: marlin/modules

(import (only-in :clan/poo/object .o)
        :marlin/modules/policy-object)

(export marlin-subagent-policy-family
        marlin-context-compression-policy-family
        marlin-tool-batch-policy-family
        marlin-self-evolution-policy-family
        marlinSubagentPolicy
        marlinContextCompressionPolicy
        marlinToolBatchPolicy
        marlinSelfEvolutionPolicy
        marlinDefaultSubagentPolicy
        marlinDefaultContextCompressionPolicy
        marlinDefaultToolBatchPolicy
        marlinDefaultSelfEvolutionPolicy)

;;; Boundary: Subagent policy is Scheme strategy; Rust owns process spawning.
;; String
(def marlin-subagent-policy-family
  "subagent-policy")

;;; Boundary: Context compression policy is Scheme selection over Rust budgets.
;; String
(def marlin-context-compression-policy-family
  "context-compression-policy")

;;; Boundary: Tool batching is Scheme strategy over Rust tool execution.
;; String
(def marlin-tool-batch-policy-family
  "tool-batch-policy")

;;; Boundary: Self-evolution policy proposes changes; Rust owns application.
;; String
(def marlin-self-evolution-policy-family
  "self-evolution-policy")

;;; Boundary: User-authored .ss files extend subagent behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinSubagentPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "subagent-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-subagent-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: User-authored .ss files extend compression as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinContextCompressionPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "context-compression-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-context-compression-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: User-authored .ss files extend tool batching as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinToolBatchPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "tool-batch-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-tool-batch-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: User-authored .ss files extend self-evolution as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinSelfEvolutionPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "self-evolution-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-self-evolution-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished subagent policy declares intent, not process plumbing.
;; PolicyObject <- Void
(def (marlinDefaultSubagentPolicy)
  (marlinSubagentPolicy
   "default-subagent"
   (.o spawn-mode: "catalog-gated"
       isolation: "workspace-scoped"
       route-owner: "poo-flow.scheme"
       runtime-owner: "rust")
   '((owner . "marlin") (surface . "default-subagent-policy"))))

;;; Boundary: Furnished compression policy declares trigger and target only.
;; PolicyObject <- Void
(def (marlinDefaultContextCompressionPolicy)
  (marlinContextCompressionPolicy
   "default-context-compression"
   (.o trigger: "budget-pressure"
       strategy: "receipt-first-summary"
       projection-target: "context-compression-receipt"
       budget-owner: "rust")
   '((owner . "marlin") (surface . "default-context-compression-policy"))))

;;; Boundary: Furnished batching policy names limits; Rust executes tools.
;; PolicyObject <- Void
(def (marlinDefaultToolBatchPolicy)
  (marlinToolBatchPolicy
   "default-tool-batch"
   (.o batch-mode: "bounded"
       scheduling: "receipt-ordered"
       execution-owner: "rust"
       budget-owner: "rust")
   '((owner . "marlin") (surface . "default-tool-batch-policy"))))

;;; Boundary: Furnished self-evolution stays proposal-only by default.
;; PolicyObject <- Void
(def (marlinDefaultSelfEvolutionPolicy)
  (marlinSelfEvolutionPolicy
   "default-self-evolution"
   (.o mode: "proposal-only"
       review: "human-review-required"
       apply-owner: "rust"
       receipt-owner: "poo-flow.scheme")
   '((owner . "marlin") (surface . "default-self-evolution-policy"))))
