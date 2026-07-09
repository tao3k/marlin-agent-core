;;; -*- Gerbil -*-
;;; Boundary: Example test runs the user-interface workspace workflow.

(import :clan/poo/object
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-option-config-id
                 poo-flow-module-option-config-value
                 poo-flow-user-module-selection-key
                 poo-flow-user-module-selection-flags
                 poo-flow-module-system-owner
                 poo-flow-sandbox-profile-by-name
                 poo-flow-sandbox-profile-capabilities
                 poo-flow-scheme-owner)
        :config-interface/modules/lib
        :config-interface/modules/prefabs/user-interface
        :config-interface/modules/prefabs/user-interface-delivery
        :marlin/deck-runtime-script
        :marlin/deck-runtime-strategy
        :marlin/graph-loop-continuation-native-projection
        (only-in :custom/marlin-user-interface/config
                 poo-flow-custom-module-workspace-module
                 poo-flow-custom-module-session-module
                 poo-flow-custom-module-loops-module
                 poo-flow-custom-module-user-interface-case
                 poo-flow-custom-module-funflow-case)
        (only-in "../init"
                 poo-flow-user-module-bundles)
        :std/test)

(export #t)

;;; Boundary: The example is not installed as a Gerbil package, so poo-flow
;;; load! falls back to generic generated names. The adapter owns semantic
;;; aliases; the user config remains only load! declarations.
(def poo-flow-custom-marlin-user-interface-workspace-module
  poo-flow-custom-module-workspace-module)
(def poo-flow-custom-marlin-user-interface-session-module
  poo-flow-custom-module-session-module)
(def poo-flow-custom-marlin-user-interface-loop-engine-module
  poo-flow-custom-module-loops-module)
(def poo-flow-custom-marlin-user-interface-user-interface-case
  poo-flow-custom-module-user-interface-case)
(def poo-flow-custom-marlin-user-interface-funflow-case
  poo-flow-custom-module-funflow-case)

(def user-interface-init-selection-keys
  (UserInterfaceRootSelectionKeys poo-flow-user-module-bundles))

;;; Boundary: User-owned selections mirror poo-flow/user-interface/custom.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-workspace-selection
  (UserInterfaceModuleBundleSelection
   poo-flow-custom-marlin-user-interface-workspace-module))

;;; Boundary: Adapter layer turns the generic selection into a Marlin prefab.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-workspace
  (UserInterfaceWorkspaceFromModuleBundle
   poo-flow-custom-marlin-user-interface-workspace-module))

;;; Boundary: One furnished entrypoint remains the adapter output.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface)
  (UserInterface user-interface-workspace))

;;; Boundary: Delivery receipt is the Rust/debug handoff.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-delivery-receipt)
  (UserInterfaceDeliveryReceipt user-interface-workspace))

;;; Boundary: Policy apply is the delivery action.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-policy-apply)
  (UserInterfaceApply user-interface-workspace))

;;; Boundary: Policy projection is the fixed Rust envelope.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-policy-projection)
  (UserInterfacePolicyProjection user-interface-workspace))

;;; Boundary: Loop control-plane stays a poo-flow handoff manifest.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-loop-governor-runtime-manifest)
  (UserInterfaceLoopGovernorRuntimeManifest user-interface-workspace))

;;; Boundary: Fixture context models a downstream user command.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-context
  (.o command: "marlin user-interface workflow apply"
      agent-scope: "user-interface-agent"
      workspace-root: "user-interface-workspace"))

;;; Boundary: Upstream workflow utility owns projections from user config.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-workflow
  (UserInterfaceWorkspaceWorkflow user-interface-workspace))

;;; Boundary: Public catalog entrypoint is visible to downstream users.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-catalog
  (UserInterfaceWorkspaceCatalog user-interface-workspace))

;;; Boundary: evalModules is the public poo-flow evaluation entrypoint.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-eval-result
  (UserInterfaceWorkspaceEvaluation user-interface-workspace))

;;; Boundary: Presentation receipt summarizes the complete policy facade surface.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-presentation
  (UserInterfaceWorkspacePolicyFacadePresentation user-interface-workspace))

;;; Boundary: Prefab presentation shows module-pack object surgery to users.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-pack-presentation
  (UserInterfacePolicyPackPresentation
   (UserInterfacePolicyPack user-interface-workspace)))

;;; Boundary: Downstream user-facing handoff is a single delivery receipt.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-delivery
  (user-interface-delivery-receipt))

;;; Boundary: Downstream can use one furnished entrypoint without plumbing.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-thin-entrypoint
  (user-interface))

;;; Boundary: Downstream user-facing apply is the delivery action.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-apply
  (user-interface-policy-apply))

;;; Boundary: Downstream user-facing projection is the Rust envelope.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-projection
  (user-interface-policy-projection))

;;; Boundary: Downstream sees the loop handoff as a furnished manifest.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-governor-manifest
  (user-interface-loop-governor-runtime-manifest))

;;; Boundary: Marlin owns the loop policy; poo-flow projects the handoff.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-marlin-loops-policy
  (UserInterfaceMarlinLoopsPolicy user-interface-workspace))

;;; Boundary: Evaluation is projected by the upstream workflow utility.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-evaluation
  (.get user-interface-workflow evaluation))

(def (user-interface-find-script evaluation script-id)
  (find (lambda (script)
          (string=? (.get script id) script-id))
        (.get evaluation scripts)))

(def (user-interface-run-script evaluation script-id context)
  (let (script (user-interface-find-script evaluation script-id))
    (and script
         (marlin-deck-runtime-script-run script context))))

(def (user-interface-script-interface-receipts evaluation)
  (map marlin-deck-runtime-script-interface-receipt
       (.get evaluation scripts)))

;;; Boundary: The script is executed by real gxtest from the workspace cwd.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-result
  (user-interface-run-script
   user-interface-evaluation
   "user-interface-worker-script"
   user-interface-context))

;;; Boundary: Interface receipt crosses back to Rust as typed values.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-receipt
  (car
   (user-interface-script-interface-receipts user-interface-evaluation)))

;;; Boundary: Batch metrics are measured in Scheme and budgeted by Rust.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-metrics
  (marlin-deck-runtime-script-batch-metrics
   (user-interface-find-script
    user-interface-evaluation
    "user-interface-worker-script")
   128
   user-interface-context))

;;; Boundary: Extension catalog proves agent-authored extension objects land.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-extension-catalog
  (.get user-interface-workflow extension-catalog))

;;; Boundary: Continuation projection is built from a downstream POO profile.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-loop-continuation-action
  (.get user-interface-loop-continuation-projection action))

;;; Boundary: Subagent launch policy stays an extension receipt.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-subagent-receipt
  (marlin-deck-runtime-extension-policy-receipt
   user-interface-extension-catalog
   user-interface-subagent-context
   user-interface-subagent-route-policy
   "marlin user-interface spawn-subagent"
   "user-interface-agent"))

;;; Boundary: Option lookup keeps assertions stable as examples grow.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def (user-interface-option option-id)
  (find (lambda (option)
          (string=? (poo-flow-module-option-config-id option) option-id))
        (.get user-interface-evaluation options)))

(def user-interface-session-selection
  (UserInterfaceModuleBundleSelection
   poo-flow-custom-marlin-user-interface-session-module))

(def user-interface-session-profiles
  (UserInterfaceModuleBundleConfig
   poo-flow-custom-marlin-user-interface-session-module))

(def user-interface-session-profile
  (poo-flow-sandbox-profile-by-name
   user-interface-session-profiles
   'marlin-user-interface/session))

(def user-interface-loop-engine-selection
  (UserInterfaceModuleBundleSelection
   poo-flow-custom-marlin-user-interface-loop-engine-module))

(def user-interface-loop-engine-config
  (UserInterfaceModuleBundleConfig
   poo-flow-custom-marlin-user-interface-loop-engine-module))

(def user-interface-loop-engine-profile
  (car user-interface-loop-engine-config))

(def user-interface-loop-engine-runtime
  (.get user-interface-loop-engine-profile runtime))

(def user-interface-loop-engine-budget
  (.get user-interface-loop-engine-profile budget))

(def user-interface-loop-profile-source
  "custom/marlin-user-interface/profiles/loops.ss")

(def user-interface-case-selection
  (UserInterfaceModuleBundleSelection
   poo-flow-custom-marlin-user-interface-user-interface-case))

(def user-interface-funflow-selection
  (UserInterfaceModuleBundleSelection
   poo-flow-custom-marlin-user-interface-funflow-case))

(def user-interface-funflow-config
  (UserInterfaceModuleBundleConfig
   poo-flow-custom-marlin-user-interface-funflow-case))

(def user-interface-funflow-map
  (car user-interface-funflow-config))
