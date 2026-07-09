;;; -*- Gerbil -*-
;;; Boundary: Test the upstream furnished user-interface prefab entrypoint.

(import :clan/poo/object
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-system-owner
                 poo-flow-scheme-owner)
        :config-interface/lib
        :config-interface/modules/prefabs/user-interface
        :config-interface/modules/prefabs/user-interface-delivery)

;;; Boundary: Local assertions stay scalar around POO-heavy values.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

;;; Boundary: Downstream user config is the whole public surface.
;; MarlinResult <- MarlinInput
(def ui-prefab-workspace
  (UserInterfaceWorkspace
   (.o workspace-root: "prefab-workspace"
       interface-file: "interface.org"
       state-file: "state/worker-state.org"
       model-profile: "interactive")))

;;; Boundary: Delivery receipt is the single user-facing handoff surface.
;; MarlinResult <- MarlinInput
(def ui-prefab-delivery-receipt
  (UserInterfaceDeliveryReceipt ui-prefab-workspace))

;;; Boundary: UserInterface is the thinnest furnished entrypoint.
;; MarlinResult <- MarlinInput
(def ui-prefab-user-interface
  (UserInterface ui-prefab-workspace))

;;; Boundary: Apply is the user-facing delivery action.
;; MarlinResult <- MarlinInput
(def ui-prefab-policy-apply
  (UserInterfaceApply ui-prefab-workspace))

;;; Boundary: Policy projection is the fixed Rust-facing protocol envelope.
;; MarlinResult <- MarlinInput
(def ui-prefab-policy-projection
  (UserInterfacePolicyProjection ui-prefab-workspace))

;;; Boundary: Loop governor manifest is produced by poo-flow, not Marlin.
;; MarlinResult <- MarlinInput
(def ui-prefab-loop-governor-manifest
  (UserInterfaceLoopGovernorRuntimeManifest ui-prefab-workspace))

(check (.get ui-prefab-workspace kind) => marlin-config-interface-kind)
(check (.get ui-prefab-delivery-receipt kind)
       => user-interface-delivery-receipt-kind)
(check (.get ui-prefab-user-interface kind)
       => user-interface-delivery-receipt-kind)
(check (.get ui-prefab-delivery-receipt workspace-kind)
       => marlin-config-interface-kind)
(check (.get ui-prefab-delivery-receipt policy-facade-presentation-kind)
       => marlin-policy-facade-presentation-kind)
(check (.get ui-prefab-delivery-receipt policy-facade-projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get ui-prefab-delivery-receipt policy-pack-presentation-kind)
       => marlin-policy-pack-presentation-kind)
(check (.get ui-prefab-delivery-receipt pack-catalog-kind)
       => marlin-pack-catalog-kind)
(check (.get ui-prefab-delivery-receipt pack-catalog-presentation-kind)
       => marlin-pack-catalog-presentation-kind)
(check (.get ui-prefab-delivery-receipt pack-count) => 1)
(check (.get ui-prefab-delivery-receipt pack-ids)
       => '("user-interface-prefab-pack"))
(check (.get ui-prefab-delivery-receipt policy-projection-kind)
       => marlin-policy-projection-kind)
(check (.get ui-prefab-delivery-receipt policy-projection-receipt-kind)
       => marlin-policy-projection-kind)
(check (.get ui-prefab-delivery-receipt policy-projection-chain-receipt-kind)
       => marlin-policy-projection-chain-receipt-kind)
(check (.get ui-prefab-delivery-receipt native-projection-payload-kind)
       => marlin-policy-pack-presentation-kind)
(check (.get ui-prefab-delivery-receipt budget-receipt-kind)
       => marlin-policy-budget-receipt-kind)
(check (.get ui-prefab-delivery-receipt catalog-resolution-receipt-kind)
       => marlin-policy-catalog-resolution-receipt-kind)
(check (.get ui-prefab-delivery-receipt projection-receipt-family-count)
       => 5)
(check (.get ui-prefab-delivery-receipt projection-receipt-family-ids)
       => '("module_evaluation_receipt"
            "policy_projection_receipt"
            "native_projection_payload"
            "budget_receipt"
            "catalog_resolution_receipt"))
(check (.get ui-prefab-delivery-receipt module-evaluation-receipt-owner)
       => poo-flow-module-system-owner)
(check (.get ui-prefab-delivery-receipt policy-projection-receipt-owner)
       => poo-flow-scheme-owner)
(check (.get ui-prefab-delivery-receipt native-projection-payload-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt budget-receipt-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt catalog-resolution-receipt-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt catalog-resolution-allowed-hook-count)
       => 2)
(check (.get ui-prefab-delivery-receipt root-module-id)
       => "user-interface-root-module")
(check (.get ui-prefab-delivery-receipt root-module-kind)
       => marlin-config-interface-kind)
(check (.get ui-prefab-delivery-receipt module-count) => 7)
(check (.get ui-prefab-delivery-receipt extension-count) => 2)
(check (.get ui-prefab-delivery-receipt script-count) => 1)
(check (.get ui-prefab-delivery-receipt option-count) => 16)
(check (.get ui-prefab-delivery-receipt validation-receipt-count) => 16)
(check (.get ui-prefab-delivery-receipt option-contract-count) => 16)
(check (.get ui-prefab-delivery-receipt pack-id)
       => "user-interface-prefab-pack")
(check (.get ui-prefab-delivery-receipt policy-object-count) => 22)
(check (.get ui-prefab-delivery-receipt default-policy-object-count) => 22)
(check (.get ui-prefab-delivery-receipt disabled-policy-object-count) => 1)
(check (.get ui-prefab-delivery-receipt object-operation-count) => 4)
(check (.get ui-prefab-delivery-receipt object-surgery-receipt-count) => 4)
(check (.get ui-prefab-delivery-receipt conflict-surgery-receipt-count)
       => 0)
(check (.get ui-prefab-delivery-receipt policy-object-ids)
       => '("default-workspace"
            "default-session"
            "default-agent"
            "default-hook"
            "default-model-route"
            "default-continuation"
            "default-human-review"
            "default-evidence-graph"
            "default-failure-recovery"
            "default-memory-recall"
            "default-memory-trigger"
            "default-memory-retention"
            "default-memory-visibility"
            "default-subagent"
            "default-context-compression"
            "default-tool-batch"
            "default-self-evolution"
            "default-catalog-projection"
            "user-interface-subagent-policy-extension"
            "user-interface-continuation-projection"
            "user-interface-marlin-loops-policy"
            "user-interface-memory-trigger"))
(check (.get ui-prefab-delivery-receipt default-policy-object-ids)
       => '("default-workspace"
            "default-session"
            "default-agent"
            "default-hook"
            "default-model-route"
            "default-continuation"
            "default-human-review"
            "default-evidence-graph"
            "default-failure-recovery"
            "default-memory-recall"
            "default-memory-trigger"
            "default-memory-retention"
            "default-memory-visibility"
            "default-subagent"
            "default-context-compression"
            "default-tool-batch"
            "default-self-evolution"
            "default-catalog-projection"
            "user-interface-subagent-policy-extension"
            "user-interface-loop-continuation"
            "runtime-catalog-user-interface-hook"
            "user-interface-marlin-loops-policy"))
(check (.get ui-prefab-delivery-receipt disabled-policy-object-ids)
       => '("user-interface-subagent-policy-extension"))
(check (.get ui-prefab-delivery-receipt policy-families)
       => '("workspace-policy"
            "session-policy"
            "agent-policy"
            "hook-selection-policy"
            "model-route-policy"
            "continuation-profile-policy"
            "human-review-policy"
            "evidence-graph-policy"
            "failure-recovery-policy"
            "memory-recall-policy"
            "memory-trigger-policy"
            "memory-retention-policy"
            "memory-visibility-policy"
            "subagent-policy"
            "context-compression-policy"
            "tool-batch-policy"
            "self-evolution-policy"
            "catalog-projection-policy"
            "continuation-profile"
            "loop-engine-policy"))
(check (.get ui-prefab-delivery-receipt allowed-hook-ids)
       => '("runtime-catalog-default-hook"
            "runtime-catalog-user-interface-hook"))
(check (map (lambda (contract)
              (list (cdr (assq 'option-id contract))
                    (cdr (assq 'contract-kind contract))
                    (cdr (assq 'value-type-label contract))
                    (cdr (assq 'valid? contract))
                    (cdr (assq 'schema-owner contract))))
            (.get ui-prefab-delivery-receipt option-contracts))
       => '(("workspace-root" "default" String #t
             "user-interface-workspace-module")
            ("interface-file" "default" String #t
             "user-interface-workspace-module")
            ("state-file" "default" String #t
             "user-interface-workspace-module")
            ("agent-scope" "default" String #t
             "user-interface-agent-module")
            ("agent-class" "default" String #t
             "user-interface-agent-module")
            ("model-profile" "default" String #t
             "user-interface-agent-module")
            ("continuation-profile" "default" String #t
             "user-interface-loop-continuation-module")
            ("layer" "optional" String #t
             "user-interface-base-module")
            ("surface" "constant" String #t
             "user-interface-base-module")
            ("layer" "optional" String #t
             "user-interface-script-module")
            ("entry" "constant" String #t
             "user-interface-script-module")
            ("hook-id" "default" String #t
             "user-interface-hook-module")
            ("hook-action" "default" String #t
             "user-interface-hook-module")
            ("hook-owner" "default" String #t
             "user-interface-hook-module")
            ("surface" "constant" String #t
             "user-interface-root-module")
            ("entry" "constant" String #t
             "user-interface-root-module")))
(check (.get ui-prefab-delivery-receipt import-graph-owner)
       => poo-flow-module-system-owner)
(check (.get ui-prefab-delivery-receipt option-merge-owner)
       => poo-flow-module-system-owner)
(check (.get ui-prefab-delivery-receipt extension-composition-owner)
       => poo-flow-module-system-owner)
(check (.get ui-prefab-delivery-receipt policy-composition-owner)
       => poo-flow-scheme-owner)
(check (.get ui-prefab-delivery-receipt native-projection-payload-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt budget-receipt-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt catalog-resolution-receipt-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt runtime-lifecycle-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt loop-control-plane-owner)
       => "poo-flow")
(check (.get ui-prefab-delivery-receipt
             loop-control-plane-runtime-manifest-schema)
       => 'poo-flow.loop-governor.marlin-runtime-manifest.v1)
(check (.get ui-prefab-delivery-receipt loop-control-plane-request-schema)
       => 'poo-flow.loop-governor.marlin-request.v1)
(check (.get ui-prefab-delivery-receipt loop-control-plane-receipt-schema)
       => 'poo-flow.loop-governor.l1-run-receipt.v1)
(check (.get ui-prefab-delivery-receipt loop-control-plane-abi-schema)
       => 'poo-flow.loop-governor.marlin-abi.v1)
(check (.get ui-prefab-delivery-receipt loop-control-plane-operation)
       => 'govern-loop)
(check (.get ui-prefab-delivery-receipt loop-control-plane-target)
       => 'marlin-agent-core)
(check (.get ui-prefab-delivery-receipt loop-control-plane-transport)
       => 'scheme-abi)
(check (.get ui-prefab-delivery-receipt loop-control-plane-control-owner)
       => 'gerbil)
(check (.get ui-prefab-delivery-receipt loop-control-plane-execution-owner)
       => 'marlin-agent-core)
(check (.get ui-prefab-delivery-receipt loop-control-plane-open-patterns)
       => '(user-interface-policy-loop))
(check (.get ui-prefab-delivery-receipt loop-control-plane-blocked-patterns)
       => '())
(check (.get ui-prefab-delivery-receipt loop-control-plane-status)
       => 'handoff-ready)
(check (.get ui-prefab-delivery-receipt marlin-loops-policy-kind)
       => user-interface-marlin-loops-policy-kind)
(check (.get ui-prefab-delivery-receipt marlin-loops-policy-id)
       => "user-interface-marlin-loops-policy")
(check (.get ui-prefab-delivery-receipt marlin-loops-policy-owner)
       => "marlin")
(check (.get ui-prefab-delivery-receipt marlin-loops-policy-source)
       => "config-interface/modules/prefabs/user-interface#loops-policy")
(check (.get ui-prefab-delivery-receipt
             marlin-loops-policy-control-plane-owner)
       => "poo-flow")
(check (.get ui-prefab-delivery-receipt
             marlin-loops-policy-runtime-execution-owner)
       => "marlin-agent-core")
(check (.get ui-prefab-delivery-receipt
             marlin-loops-policy-receipt-family-count)
       => 11)
(check (.get ui-prefab-delivery-receipt
             marlin-loops-policy-receipt-contract-count)
       => 11)
(check (.get ui-prefab-delivery-receipt
             marlin-loops-policy-receipt-schema-ids)
       => '(poo-flow.loop-engine.lineage-receipt.v1
            poo-flow.loop-engine.selector-receipt.v1
            poo-flow.modules.session.agent-graph.v1
            poo-flow.loop-engine.resource-dispatch-receipt.v1
            poo-flow.loop-engine.capability-receipt.v1
            poo-flow.loop-engine.memory-receipt.v1
            poo-flow.loop-engine.compression-receipt.v1
            poo-flow.loop-engine.policy-extension-receipt.v1
            poo-flow.spec-evolution.review-item.v1
            poo-flow.spec-evolution.runtime-manifest-row.v1
            poo-flow.loop-engine.sandbox-handoff-agreement.v1))
(check (.get ui-prefab-delivery-receipt
             marlin-loops-policy-receipt-contract-owners)
       => '("poo-flow"))
(check (.get ui-prefab-delivery-receipt rust-parses-scheme-source)
       => #f)
(check (.get ui-prefab-delivery-receipt rust-handler-manufactured)
       => #f)
(check (.get ui-prefab-delivery-receipt replayable) => #t)
(check (.get ui-prefab-delivery-receipt user-entrypoints)
       => '("UserInterfaceWorkspace"
            "UserInterfaceLoopGovernorRuntimeManifest"
            "UserInterfaceDeliveryReceipt"
            "UserInterfaceApply"
            "UserInterfacePolicyProjection"))
(check (cdr (assq 'schema ui-prefab-loop-governor-manifest))
       => 'poo-flow.loop-governor.marlin-runtime-manifest.v1)
(check (cdr (assq 'producer ui-prefab-loop-governor-manifest))
       => 'poo-flow)
(check (cdr (assq 'consumer ui-prefab-loop-governor-manifest))
       => 'marlin-agent-core)
(check (cdr (assq 'operation ui-prefab-loop-governor-manifest))
       => 'govern-loop)
(check (.get ui-prefab-policy-apply kind)
       => user-interface-delivery-receipt-kind)
(check (.get ui-prefab-policy-projection kind)
       => marlin-policy-projection-kind)
(check (.get ui-prefab-policy-projection pack-id)
       => "user-interface-prefab-pack")
(check (.get ui-prefab-policy-projection native-projection-payload-kind)
       => marlin-policy-pack-presentation-kind)
(check (.get ui-prefab-policy-projection runtime-lifecycle-owner)
       => "rust")
