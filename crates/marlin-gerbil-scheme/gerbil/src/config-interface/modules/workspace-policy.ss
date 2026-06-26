;;; -*- Gerbil -*-
;;; Boundary: Workspace policy objects for prefab packs.

package: config-interface/modules

(import (only-in :clan/poo/object .o)
        :config-interface/modules/policy-object)

(export marlin-workspace-policy-family
        marlinWorkspacePolicy
        marlinDefaultWorkspacePolicy)

;;; Boundary: Workspace policy is Scheme strategy over Rust receipts.
;; String
(def marlin-workspace-policy-family
  "workspace-policy")

;;; Boundary: User-authored .ss files extend workspace behavior as POO objects.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinWorkspacePolicy object-id-value payload-value . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "workspace-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-workspace-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished workspace policy names sharing and snapshot intent.
;; PolicyObject <- Void
(def (marlinDefaultWorkspacePolicy)
  (marlinWorkspacePolicy
   "default-workspace"
   (.o isolation: "shared-worktree"
       branch-mode: "policy-branch"
       snapshot-mode: "typed-receipt-snapshot")
   '((owner . "marlin") (surface . "default-workspace-policy"))))
