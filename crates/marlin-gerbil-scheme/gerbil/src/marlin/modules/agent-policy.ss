;;; -*- Gerbil -*-
;;; Boundary: Agent policy objects for prefab packs.

package: marlin/modules

(import (only-in :clan/poo/object .o)
        :marlin/modules/policy-object)

(export marlin-agent-policy-family
        marlinAgentPolicy
        marlinDefaultAgentPolicy)

;;; Boundary: Agent policy is Scheme strategy; Rust owns process execution.
;; String
(def marlin-agent-policy-family
  "agent-policy")

;;; Boundary: User-authored .ss files extend agent behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinAgentPolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "agent-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-agent-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished agent policy keeps subagent choice in Scheme.
;; PolicyObject <- Void
(def (marlinDefaultAgentPolicy)
  (marlinAgentPolicy
   "default-agent"
   (.o root-agent: "root-agent"
       subagent-policy: "module-managed")
   '((owner . "marlin") (surface . "default-agent-policy"))))
