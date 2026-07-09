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



(import "./user-interface-module-config-fixture")

(test-case "user-interface module config workflow"
(check user-interface-init-selection-keys
       => '(surface
            profile
            flow-mode
            loop-strategy
            sandbox-policy
            sandbox-backends
            mode-lock))
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
(check (.get user-interface-result command) => "marlin user-interface workflow apply")
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
       => marlin-policy-facade-presentation-kind)
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
             policy-facade-presentation-kind)
       => marlin-policy-facade-presentation-kind)
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
       => "config-interface/modules/prefabs/user-interface#loops-policy")
(check (.get user-interface-marlin-loops-policy control-plane-owner)
       => "poo-flow")
(check (.get user-interface-marlin-loops-policy runtime-execution-owner)
       => "marlin-agent-core")
(check (.get user-interface-delivery marlin-loops-policy-owner)
       => "marlin")
(check (.get user-interface-delivery marlin-loops-policy-source)
       => "config-interface/modules/prefabs/user-interface#loops-policy")
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
)
