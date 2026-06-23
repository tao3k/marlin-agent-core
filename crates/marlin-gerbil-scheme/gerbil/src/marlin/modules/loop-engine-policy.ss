;;; -*- Gerbil -*-
;;; Boundary: Loop-engine policy objects for prefab packs.

package: marlin/modules

(import (only-in :clan/poo/object .o)
        :marlin/modules/policy-object)

(export marlin-loop-engine-policy-family
        marlinDefaultLoopEngineReceiptContracts
        marlinLoopEnginePolicy
        marlinDefaultLoopEnginePolicy)

;;; Boundary: Loop policy is Marlin-owned policy furniture; poo-flow projects it.
;; String
(def marlin-loop-engine-policy-family
  "loop-engine-policy")

;;; Boundary: Marlin consumes these poo-flow loop receipts as typed contracts.
;; [Alist]
(def (marlinDefaultLoopEngineReceiptContracts)
  '(((id . "lineage-receipt")
     (schema . poo-flow.loop-engine.lineage-receipt.v1)
     (owner . "poo-flow"))
    ((id . "selector-receipt")
     (schema . poo-flow.loop-engine.selector-receipt.v1)
     (owner . "poo-flow"))
    ((id . "resource-dispatch-receipt")
     (schema . poo-flow.loop-engine.resource-dispatch-receipt.v1)
     (owner . "poo-flow"))
    ((id . "capability-receipt")
     (schema . poo-flow.loop-engine.capability-receipt.v1)
     (owner . "poo-flow"))
    ((id . "memory-receipt")
     (schema . poo-flow.loop-engine.memory-receipt.v1)
     (owner . "poo-flow"))
    ((id . "compression-receipt")
     (schema . poo-flow.loop-engine.compression-receipt.v1)
     (owner . "poo-flow"))
    ((id . "policy-extension-receipt")
     (schema . poo-flow.loop-engine.policy-extension-receipt.v1)
     (owner . "poo-flow"))
    ((id . "sandbox-handoff-agreement")
     (schema . poo-flow.loop-engine.sandbox-handoff-agreement.v1)
     (owner . "poo-flow"))))

;;; Boundary: User-authored .ss files can extend loop behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinLoopEnginePolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "loop-engine-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-loop-engine-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Default loop policy is inert until a prefab requests a handoff.
;; PolicyObject <- Void
(def (marlinDefaultLoopEnginePolicy)
  (marlinLoopEnginePolicy
   "default-loop-engine"
   (.o mode: "report-only"
       control-plane-owner: "poo-flow"
       runtime-execution-owner: "marlin-agent-core"
       runtime-effect: "handoff-only"
       receipt-contracts: (marlinDefaultLoopEngineReceiptContracts))
   '((owner . "marlin") (surface . "default-loop-engine-policy"))))
