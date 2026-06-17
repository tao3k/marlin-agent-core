;;; -*- Gerbil -*-
;;; Boundary: Advanced framework-authoring example for raw policy packs.

(import :clan/poo/object
        :modules/lib)

(export framework-authoring-module
        framework-authoring-policy-pack
        framework-authoring-policy-projection
        framework-authoring-projection-chain)

;;; Boundary: Framework authors may define a module shell directly.
;; MarlinResult <- MarlinInput
(def framework-authoring-module-interface
  (marlin-module-interface
   "FrameworkAuthoringModule"
   (.o surface: (marlin-string-constant "framework-authoring"))
   '((owner . "framework-authoring-example"))))

;;; Boundary: Raw marlinModules is an advanced API, not the default user path.
;; MarlinResult <- MarlinInput
(def framework-authoring-module
  (marlinModules
   framework-authoring-module-interface
   (.o id: "framework-authoring-root-module"
       config: (.o surface: "framework-authoring"))))

;;; Boundary: Advanced callers compose domain policy objects directly.
;; MarlinResult <- MarlinInput
(def framework-context-compression-policy
  (marlinContextCompressionPolicy
   "framework-context-compression"
   (.o trigger: "large-context"
       strategy: "evidence-linked-summary"
       projection-target: "context-compression-receipt")
   '((owner . "framework-authoring-example"))))

;;; Boundary: Tool batch strategy remains POO data before Rust execution.
;; MarlinResult <- MarlinInput
(def framework-tool-batch-policy
  (marlinToolBatchPolicy
   "framework-tool-batch"
   (.o batch-mode: "ordered"
       scheduling: "dependency-aware"
       execution-owner: "rust")
   '((owner . "framework-authoring-example"))))

;;; Boundary: Self-evolution stays proposal-only in the example pack.
;; MarlinResult <- MarlinInput
(def framework-self-evolution-policy
  (marlinSelfEvolutionPolicy
   "framework-self-evolution"
   (.o mode: "proposal-only"
       review: "human-required"
       apply-owner: "rust")
   '((owner . "framework-authoring-example"))))

;;; Boundary: Subagent policy names strategy; runtime spawn stays Rust-owned.
;; MarlinResult <- MarlinInput
(def framework-subagent-policy
  (marlinSubagentPolicy
   "framework-subagent"
   (.o spawn-mode: "catalog-gated"
       isolation: "branch-scoped"
       runtime-owner: "rust")
   '((owner . "framework-authoring-example"))))

;;; Boundary: Raw pack construction is reserved for framework authors.
;; MarlinResult <- MarlinInput
(def framework-authoring-policy-pack
  (marlinPolicyPack
   (.o id: "framework-authoring-custom-pack"
       module: framework-authoring-module
       policy-objects:
       (list framework-context-compression-policy
             framework-tool-batch-policy
             framework-self-evolution-policy
             framework-subagent-policy)
       object-operations:
       (list
        (marlin-disable-object
         "self-evolution-policy"
         "framework-self-evolution"
         "example keeps self-evolution proposal-only"))
       allowed-hook-ids: '("framework-authoring-existing-hook")
       metadata: '((owner . "framework-authoring-example")
                   (surface . "raw-policy-pack")))))

;;; Boundary: Framework authors still hand Rust a typed projection envelope.
;; MarlinResult <- MarlinInput
(def framework-authoring-policy-projection
  (marlinPolicyProjection framework-authoring-policy-pack))

;;; Boundary: The same five-family chain applies to raw packs.
;; MarlinResult <- MarlinInput
(def framework-authoring-projection-chain
  (marlinPolicyProjectionChainReceipt framework-authoring-policy-pack))
