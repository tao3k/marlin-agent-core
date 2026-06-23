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
        :marlin/modules/lib
        :marlin/modules/prefabs/user-interface
        :marlin/modules/prefabs/user-interface-delivery
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
  (.o command: "codex user-interface workflow apply"
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

;;; Boundary: evalModules is the public module-system evaluation entrypoint.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-eval-result
  (UserInterfaceWorkspaceEvaluation user-interface-workspace))

;;; Boundary: Presentation receipt summarizes the complete module-system surface.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-presentation
  (UserInterfaceWorkspaceSystemPresentation user-interface-workspace))

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
   "codex user-interface spawn-subagent"
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

(check user-interface-init-selection-keys
       => '((flow . funflow)
            (loop . governor)
            (sandbox . nono-sandbox)
            (sandbox . cubeSandbox)
            (sandbox . docker-sandbox)
            (flow . loop-engine)
            (custom . marlin-user-interface)))
(check (poo-flow-user-module-selection-key
        (UserInterfaceRootSelection
         poo-flow-user-module-bundles
         '(custom . marlin-user-interface)))
       => '(custom . marlin-user-interface))
(check (length
        (UserInterfaceModuleBundleSelections
         poo-flow-custom-marlin-user-interface-workspace-module))
       => 1)
(check (poo-flow-user-module-selection-key user-interface-workspace-selection)
       => '(custom . marlin-user-interface))
(check (poo-flow-user-module-selection-flags user-interface-workspace-selection)
       => '((workspace-root . "user-interface-workspace")
            (interface-file . "interface.org")
            (state-file . "state/worker-state.org")
            (model-profile . "interactive")
            (hook-id . "runtime-catalog-user-interface-hook")
            (hook-action . "register")
            (hook-owner . "user-interface-worker")
            (continuation-profile . "user-interface-loop-continuation")))
(check (poo-flow-user-module-selection-key user-interface-session-selection)
       => '(sandbox . nono-sandbox))
(check (length user-interface-session-profiles) => 1)
(check (poo-flow-sandbox-profile-capabilities user-interface-session-profile)
       => '(process-run filesystem-read tmpdir cache-mount))
(check (poo-flow-user-module-selection-key user-interface-loop-engine-selection)
       => '(flow . loop-engine))
(check (length user-interface-loop-engine-config) => 1)
(check (.get user-interface-loop-engine-runtime capabilities)
       => '(+manifest-handoff +l1-receipts))
(check (.get user-interface-loop-engine-runtime handoff)
       => 'loop-governor-marlin-runtime-manifest)
(check (.get user-interface-loop-engine-runtime owner)
       => "marlin-agent-core")
(check (.get user-interface-loop-engine-runtime runtime-executed)
       => #f)
(check user-interface-loop-profile-source
       => "custom/marlin-user-interface/profiles/loops.ss")
(check (.get user-interface-loop-engine-budget max-actionable) => 1)
(check (.get user-interface-loop-engine-budget max-attempts) => 1)
(check (poo-flow-user-module-selection-key user-interface-case-selection)
       => '(custom . marlin-user-interface))
(check (UserInterfaceModuleBundleFlag
        poo-flow-custom-marlin-user-interface-user-interface-case
        'policy-pack)
       => "user-interface-prefab-pack")
(check (poo-flow-user-module-selection-key user-interface-funflow-selection)
       => '(flow . funflow))
(check (length user-interface-funflow-config) => 1)
(check (.get user-interface-funflow-map map-name)
       => 'marlin-ui-policy-handoff)
(check (.get user-interface-funflow-map runtime-executed) => #f)
(check (cdr (assq 'execution-owner
                  (.get user-interface-funflow-map metadata)))
       => 'marlin-agent-core)
(check (.get user-interface-result command) => "codex user-interface workflow apply")
(check (.get user-interface-result agent-scope) => "user-interface-agent")
(check (.get user-interface-result has-interface-file) => #t)
(check (.get user-interface-result has-worker-state-file) => #t)
(check (.get user-interface-catalog kind) => marlin-module-catalog-kind)
(check (length (.get user-interface-catalog modules)) => 1)
(check (.get user-interface-eval-result kind)
       => marlin-eval-modules-result-kind)
(check (.get user-interface-eval-result root-module-id)
       => "user-interface-root-module")
(check (.get user-interface-eval-result workflow-kind)
       => marlin-module-workflow-kind)
(check (.get user-interface-eval-result module-count) => 7)
(check (.get user-interface-eval-result extension-count) => 2)
(check (.get user-interface-eval-result policy-extension-object-count)
       => 1)
(check (.get user-interface-eval-result script-count) => 1)
(check (.get user-interface-eval-result option-count) => 16)
(check (.get user-interface-presentation kind)
       => marlin-module-system-presentation-kind)
(check (.get user-interface-presentation root-import-count) => 5)
(check (.get user-interface-presentation root-extension-count) => 1)
(check (.get user-interface-presentation root-policy-extension-object-count)
       => 1)
(check (.get user-interface-presentation projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get user-interface-presentation module-evaluation-receipt-kind)
       => "poo-flow.modules.runtime-evaluation.v1")
(check (.get user-interface-presentation import-graph-owner)
       => poo-flow-module-system-owner)
(check (.get user-interface-presentation extension-composition-owner)
       => poo-flow-module-system-owner)
(check (.get user-interface-presentation native-projection-payload-owner)
       => "rust")
(check (.get user-interface-presentation catalog-resolution-receipt-owner)
       => "rust")
(check (.get user-interface-presentation rust-parses-scheme-source)
       => #f)
(check (.get user-interface-presentation scheme-manufactures-rust-handlers)
       => #f)
(check (.get (UserInterfacePolicyPack user-interface-workspace) kind)
       => marlin-policy-pack-kind)
(check (.get (UserInterfacePolicyPack user-interface-workspace) id)
       => "user-interface-prefab-pack")
(check (.get (UserInterfacePolicyPackCatalog
              (UserInterfacePolicyPack user-interface-workspace))
             kind)
       => marlin-pack-catalog-kind)
(check (.get user-interface-pack-presentation kind)
       => marlin-policy-pack-presentation-kind)
(check (.get user-interface-pack-presentation pack-id)
       => "user-interface-prefab-pack")
(check (.get user-interface-pack-presentation
             module-system-presentation-kind)
       => marlin-module-system-presentation-kind)
(check (.get user-interface-pack-presentation policy-object-count) => 22)
(check (.get user-interface-pack-presentation
             default-policy-object-count)
       => 22)
(check (.get user-interface-pack-presentation object-operation-count) => 4)
(check (.get user-interface-pack-presentation
             object-surgery-receipt-count)
       => 4)
(check (.get user-interface-pack-presentation add-operation-count) => 1)
(check (.get user-interface-pack-presentation remove-operation-count) => 1)
(check (.get user-interface-pack-presentation disable-operation-count) => 1)
(check (.get user-interface-pack-presentation replace-operation-count) => 1)
(check (.get user-interface-pack-presentation
             matched-surgery-receipt-count)
       => 4)
(check (.get user-interface-pack-presentation
             disabled-policy-object-count)
       => 1)
(check (if (member "loop-engine-policy"
                   (.get user-interface-pack-presentation policy-families))
         #t
         #f)
       => #t)
(check (.get user-interface-pack-presentation allowed-hook-count) => 2)
(check (.get user-interface-pack-presentation import-graph-owner)
       => poo-flow-module-system-owner)
(check (.get user-interface-pack-presentation option-merge-owner)
       => poo-flow-module-system-owner)
(check (.get user-interface-pack-presentation
             native-projection-payload-owner)
       => "rust")
(check (.get user-interface-pack-presentation
             rust-parses-scheme-source)
       => #f)
(check (.get user-interface-pack-presentation
             rust-handler-manufactured)
       => #f)
(check (.get user-interface-delivery kind)
       => user-interface-delivery-receipt-kind)
(check (.get user-interface-thin-entrypoint kind)
       => user-interface-delivery-receipt-kind)
(check (.get user-interface-delivery root-module-id)
       => "user-interface-root-module")
(check (.get user-interface-delivery module-count)
       => (length (.get user-interface-evaluation module-ids)))
(check (.get user-interface-delivery option-contract-count) => 16)
(check (.get user-interface-delivery pack-id)
       => "user-interface-prefab-pack")
(check (.get user-interface-delivery pack-catalog-presentation-kind)
       => marlin-pack-catalog-presentation-kind)
(check (.get user-interface-delivery pack-ids)
       => '("user-interface-prefab-pack"))
(check (.get user-interface-delivery policy-projection-kind)
       => marlin-policy-projection-kind)
(check (.get user-interface-delivery policy-projection-chain-receipt-kind)
       => marlin-policy-projection-chain-receipt-kind)
(check (.get user-interface-delivery budget-receipt-kind)
       => marlin-policy-budget-receipt-kind)
(check (.get user-interface-delivery catalog-resolution-receipt-kind)
       => marlin-policy-catalog-resolution-receipt-kind)
(check (.get user-interface-delivery projection-receipt-family-count) => 5)
(check (.get user-interface-delivery projection-receipt-family-ids)
       => '("module_evaluation_receipt"
            "policy_projection_receipt"
            "native_projection_payload"
            "budget_receipt"
            "catalog_resolution_receipt"))
(check (.get user-interface-delivery module-evaluation-receipt-owner)
       => poo-flow-module-system-owner)
(check (.get user-interface-delivery policy-projection-receipt-owner)
       => poo-flow-scheme-owner)
(check (.get user-interface-delivery native-projection-payload-owner)
       => "rust")
(check (.get user-interface-delivery budget-receipt-owner)
       => "rust")
(check (.get user-interface-delivery catalog-resolution-receipt-owner)
       => "rust")
(check (.get user-interface-delivery catalog-resolution-allowed-hook-count)
       => 2)
(check (.get user-interface-delivery loop-control-plane-owner)
       => "poo-flow")
(check (.get user-interface-delivery
             loop-control-plane-runtime-manifest-schema)
       => 'poo-flow.loop-governor.marlin-runtime-manifest.v1)
(check (.get user-interface-delivery loop-control-plane-request-schema)
       => 'poo-flow.loop-governor.marlin-request.v1)
(check (.get user-interface-delivery loop-control-plane-operation)
       => 'govern-loop)
(check (.get user-interface-delivery loop-control-plane-target)
       => 'marlin-agent-core)
(check (.get user-interface-delivery loop-control-plane-execution-owner)
       => 'marlin-agent-core)
(check (.get user-interface-delivery loop-control-plane-open-patterns)
       => '(user-interface-policy-loop))
(check (.get user-interface-delivery loop-control-plane-status)
       => 'handoff-ready)
(check (.get user-interface-marlin-loops-policy kind)
       => user-interface-marlin-loops-policy-kind)
(check (.get user-interface-marlin-loops-policy owner)
       => "marlin")
(check (.get user-interface-marlin-loops-policy source)
       => "marlin/modules/prefabs/user-interface#loops-policy")
(check (.get user-interface-marlin-loops-policy control-plane-owner)
       => "poo-flow")
(check (.get user-interface-marlin-loops-policy runtime-execution-owner)
       => "marlin-agent-core")
(check (.get user-interface-delivery marlin-loops-policy-owner)
       => "marlin")
(check (.get user-interface-delivery marlin-loops-policy-source)
       => "marlin/modules/prefabs/user-interface#loops-policy")
(check (.get user-interface-delivery
             marlin-loops-policy-receipt-family-count)
       => 8)
(check (.get user-interface-delivery
             marlin-loops-policy-receipt-contract-count)
       => 8)
(check (.get user-interface-delivery
             marlin-loops-policy-receipt-schema-ids)
       => '(poo-flow.loop-engine.lineage-receipt.v1
            poo-flow.loop-engine.selector-receipt.v1
            poo-flow.loop-engine.resource-dispatch-receipt.v1
            poo-flow.loop-engine.capability-receipt.v1
            poo-flow.loop-engine.memory-receipt.v1
            poo-flow.loop-engine.compression-receipt.v1
            poo-flow.loop-engine.policy-extension-receipt.v1
            poo-flow.loop-engine.sandbox-handoff-agreement.v1))
(check (.get user-interface-delivery
             marlin-loops-policy-receipt-contract-owners)
       => '("poo-flow"))
(check (.get user-interface-delivery user-entrypoints)
       => '("UserInterfaceWorkspace"
            "UserInterfaceLoopGovernorRuntimeManifest"
            "UserInterfaceDeliveryReceipt"
            "UserInterfaceApply"
            "UserInterfacePolicyProjection"))
(check (cdr (assq 'schema user-interface-loop-governor-manifest))
       => 'poo-flow.loop-governor.marlin-runtime-manifest.v1)
(check (cdr (assq 'producer user-interface-loop-governor-manifest))
       => 'poo-flow)
(check (cdr (assq 'consumer user-interface-loop-governor-manifest))
       => 'marlin-agent-core)
(check (.get user-interface-apply kind)
       => user-interface-delivery-receipt-kind)
(check (.get user-interface-projection kind)
       => marlin-policy-projection-kind)
(check (.get user-interface-projection pack-id)
       => "user-interface-prefab-pack")
(check (.get user-interface-projection native-projection-payload-kind)
       => marlin-policy-pack-presentation-kind)
(check (.get user-interface-receipt script-id) => "user-interface-worker-script")
(check (.get user-interface-receipt extension-id) => "user-interface-worker-extension")
(check (.get user-interface-metrics iterations) => 128)
(check (.get user-interface-metrics runs) => 128)
(check (.get (marlin-deck-runtime-extension-catalog-find
              user-interface-extension-catalog
              "user-interface-subagent-policy-extension")
             id)
       => "user-interface-subagent-policy-extension")
(check (.get user-interface-subagent-policy-extension policy-extension-kind)
       => marlin-policy-extension-kind)
(check (.get user-interface-subagent-policy-extension policy-extension-object)
       => #t)
(check (.get user-interface-subagent-policy-extension policy-extension-source)
       => user-interface-subagent-policy-extension-source)
(check (.get user-interface-subagent-receipt matched) => #t)
(check (.get user-interface-subagent-receipt extension-id)
       => "user-interface-subagent-policy-extension")
(check (.get user-interface-loop-continuation-projection type_id)
       => marlin-graph-loop-continuation-type-id)
(check (.get user-interface-loop-continuation-projection schema_id)
       => marlin-graph-loop-continuation-schema-id)
(check (.get user-interface-loop-continuation-action kind)
       => "continue_with_graph")
(check (.get (.get user-interface-loop-continuation-action compiled_graph) graph_id)
       => "user-interface-continuation-graph")
(check (.get (.get user-interface-subagent-receipt dynamic-hook-action) action)
       => "register")
(check (.get (.get user-interface-subagent-receipt dynamic-hook-selection) source)
       => "extension-action")
(check (.get (.get user-interface-subagent-receipt dynamic-hook-selection) selector)
       => #f)
(check (.get user-interface-evaluation module-ids)
       => '("user-interface-root-module"
            "user-interface-workspace-module"
            "user-interface-agent-module"
            "user-interface-loop-continuation-module"
            "user-interface-script-module"
            "user-interface-base-module"
            "user-interface-hook-module"))
(check (poo-flow-module-option-config-value (user-interface-option "surface"))
       => "downstream-user-interface")
(check (poo-flow-module-option-config-value (user-interface-option "entry"))
       => "interface-workflow")
(check (poo-flow-module-option-config-value (user-interface-option "workspace-root"))
       => "user-interface-workspace")
(check (poo-flow-module-option-config-value (user-interface-option "interface-file"))
       => "interface.org")
(check (poo-flow-module-option-config-value (user-interface-option "state-file"))
       => "state/worker-state.org")
(check (poo-flow-module-option-config-value (user-interface-option "agent-scope"))
       => "user-interface-agent")
(check (poo-flow-module-option-config-value (user-interface-option "agent-class"))
       => "customer-user-interface")
(check (poo-flow-module-option-config-value (user-interface-option "model-profile"))
       => "interactive")
(check (poo-flow-module-option-config-value (user-interface-option "hook-id"))
       => "runtime-catalog-user-interface-hook")
(check (poo-flow-module-option-config-value (user-interface-option "hook-action"))
       => "register")
(check (poo-flow-module-option-config-value (user-interface-option "hook-owner"))
       => "user-interface-worker")
(check (poo-flow-module-option-config-value (user-interface-option "continuation-profile"))
       => "user-interface-loop-continuation")
(check (poo-flow-module-option-config-value (user-interface-option "layer"))
       => "script")
(check (map (lambda (receipt) (.get receipt valid?))
            (.get user-interface-workflow root-validation-receipts))
       => '(#t #t))
(check (andmap (lambda (receipt) (.get receipt valid?))
               (.get user-interface-workflow validation-receipts))
       => #t)

(display "user-interface-script-workflow-ok")
(newline)
(display "script-id=")
(display (.get user-interface-receipt script-id))
(newline)
(display "extension-id=")
(display (.get user-interface-receipt extension-id))
(newline)
(display "continuation-kind=")
(display (.get user-interface-loop-continuation-action kind))
(newline)
(display "loop-profile-source=")
(display user-interface-loop-profile-source)
(newline)
(display "runtime-handoff-status=")
(display (.get user-interface-delivery loop-control-plane-status))
(newline)
(display "runtime-execution-owner=")
(display (.get user-interface-delivery loop-control-plane-execution-owner))
(newline)
(display "loops-policy-owner=")
(display (.get user-interface-delivery marlin-loops-policy-owner))
(newline)
(display "loops-policy-source=")
(display (.get user-interface-delivery marlin-loops-policy-source))
(newline)
(display "loops-policy-receipt-contract-count=")
(display (.get user-interface-delivery
               marlin-loops-policy-receipt-contract-count))
(newline)
(display "has-interface-file=")
(display (if (.get user-interface-result has-interface-file) "true" "false"))
(newline)
(display "metrics-kind=")
(display (.get user-interface-metrics kind))
(newline)
(display "metrics-interface=")
(display (.get user-interface-metrics interface))
(newline)
(display "iterations=")
(display (.get user-interface-metrics iterations))
(newline)
(display "runs=")
(display (.get user-interface-metrics runs))
(newline)
(display "elapsed_us=")
(display (.get user-interface-metrics elapsed-us))
(newline)
