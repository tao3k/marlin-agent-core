;;; -*- Gerbil -*-
;;; Boundary: Test the upstream furnished user-interface prefab entrypoint.

(import :clan/poo/object
        :modules/lib
        :modules/prefabs/user-interface
        :modules/prefabs/user-interface-delivery)

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
(def ui-prefab-config
  (UserInterfaceWorkspace
   (.o workspace-root: "prefab-workspace"
       interface-file: "interface.org"
       state-file: "state/worker-state.org"
       model-profile: "interactive")))

;;; Boundary: Delivery receipt is the single user-facing handoff surface.
;; MarlinResult <- MarlinInput
(def ui-prefab-delivery-receipt
  (UserInterfaceDeliveryReceipt ui-prefab-config))

;;; Boundary: Apply is the user-facing delivery action.
;; MarlinResult <- MarlinInput
(def ui-prefab-policy-apply
  (UserInterfaceApply ui-prefab-config))

;;; Boundary: Policy projection is the fixed Rust-facing protocol envelope.
;; MarlinResult <- MarlinInput
(def ui-prefab-policy-projection
  (UserInterfacePolicyProjection ui-prefab-config))

(check (.get ui-prefab-config kind) => marlin-modules-kind)
(check (.get ui-prefab-delivery-receipt kind)
       => user-interface-delivery-receipt-kind)
(check (.get ui-prefab-delivery-receipt workspace-kind)
       => marlin-modules-kind)
(check (.get ui-prefab-delivery-receipt module-system-presentation-kind)
       => marlin-module-system-presentation-kind)
(check (.get ui-prefab-delivery-receipt module-system-projection-chain-kind)
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
(check (.get ui-prefab-delivery-receipt root-module-id)
       => "user-interface-root-module")
(check (.get ui-prefab-delivery-receipt root-module-kind)
       => marlin-modules-kind)
(check (.get ui-prefab-delivery-receipt module-count) => 7)
(check (.get ui-prefab-delivery-receipt extension-count) => 2)
(check (.get ui-prefab-delivery-receipt script-count) => 1)
(check (.get ui-prefab-delivery-receipt option-count) => 13)
(check (.get ui-prefab-delivery-receipt validation-receipt-count) => 12)
(check (.get ui-prefab-delivery-receipt option-contract-count) => 12)
(check (.get ui-prefab-delivery-receipt pack-id)
       => "user-interface-prefab-pack")
(check (.get ui-prefab-delivery-receipt policy-object-count) => 14)
(check (.get ui-prefab-delivery-receipt default-policy-object-count) => 14)
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
            "default-memory-trigger"
            "default-catalog-projection"
            "user-interface-subagent-policy-extension"
            "user-interface-continuation-projection"
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
            "default-memory-trigger"
            "default-catalog-projection"
            "user-interface-subagent-policy-extension"
            "user-interface-loop-continuation"
            "runtime-catalog-user-interface-hook"))
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
            "memory-trigger-policy"
            "catalog-projection-policy"
            "subagent-policy-extension"
            "continuation-profile"))
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
       => "gerbil-module-system")
(check (.get ui-prefab-delivery-receipt option-merge-owner)
       => "gerbil-poo")
(check (.get ui-prefab-delivery-receipt extension-composition-owner)
       => "gerbil-poo")
(check (.get ui-prefab-delivery-receipt policy-composition-owner)
       => "gerbil-poo")
(check (.get ui-prefab-delivery-receipt native-projection-payload-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt budget-receipt-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt catalog-resolution-receipt-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt runtime-lifecycle-owner)
       => "rust")
(check (.get ui-prefab-delivery-receipt rust-parses-scheme-source)
       => #f)
(check (.get ui-prefab-delivery-receipt rust-handler-manufactured)
       => #f)
(check (.get ui-prefab-delivery-receipt replayable) => #t)
(check (.get ui-prefab-delivery-receipt user-entrypoints)
       => '("UserInterfaceWorkspace"
            "UserInterfaceDeliveryReceipt"
            "UserInterfaceApply"
            "UserInterfacePolicyProjection"))
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
