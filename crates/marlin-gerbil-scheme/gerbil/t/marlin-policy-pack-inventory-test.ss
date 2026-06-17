;;; -*- Gerbil -*-
;;; Boundary: Module tests prefab pack inventories without full runtime fixtures.

(import :clan/poo/object
        :modules/lib
        :modules/prefabs/default-policy)

;;; Boundary: Local assertions stay scalar around POO values.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

;;; Boundary: Minimal module keeps inventory tests focused on prefab objects.
;; MarlinResult <- MarlinInput
(def inventory-module-interface
  (marlin-module-interface
   "InventoryModule"
   (.o surface: (marlin-string-constant "inventory"))
   '((owner . "policy-pack-inventory-test"))))

;;; Boundary: Inventory packs still wrap a real marlinModules value.
;; MarlinResult <- MarlinInput
(def inventory-module
  (marlinModules
   inventory-module-interface
   (.o id: "inventory-module"
       config: (.o surface: "inventory"))))

;;; Boundary: Default pack is the upstream furnished entrypoint.
;; MarlinResult <- MarlinInput
(def default-pack
  (marlinDefaultPolicyPack inventory-module))

(def default-inventory
  (marlinPolicyPackInventory default-pack))

(def default-catalog
  (DefaultPolicyPackCatalog default-pack))

(def default-catalog-presentation
  (DefaultPolicyPackCatalogPresentation default-pack))

(def default-delivery
  (DefaultPolicyDeliveryReceipt inventory-module))

(def default-apply
  (DefaultPolicyApply inventory-module))

;;; Boundary: Object surgery keeps custom pack inventory deterministic.
;; MarlinResult <- MarlinInput
(def route-object
  (marlinModelRoutePolicy "route" (.o route: "normal")))

(def review-object
  (marlinHumanReviewPolicy "review" (.o reviewer: "root")))

(def hook-object
  (marlinHookSelectionPolicy "hook" (.o hook-id: "hook")))

(def memory-object
  (marlinMemoryTriggerPolicy "memory" (.o action: "compact")))

(def evidence-object
  (marlinEvidenceGraphPolicy
   "evidence"
   (.o query-family: "test-evidence-query")))

(def failure-object
  (marlinFailureRecoveryPolicy
   "failure"
   (.o query-family: "test-failure-query")))

(def default-evidence-object
  (marlinDefaultEvidenceGraphPolicy))

(def default-failure-object
  (marlinDefaultFailureRecoveryPolicy))

(def default-memory-recall-object
  (marlinDefaultMemoryRecallPolicy))

(def default-memory-trigger-object
  (marlinDefaultMemoryTriggerPolicy))

(def default-memory-retention-object
  (marlinDefaultMemoryRetentionPolicy))

(def default-memory-visibility-object
  (marlinDefaultMemoryVisibilityPolicy))

(def default-subagent-object
  (marlinDefaultSubagentPolicy))

(def default-context-compression-object
  (marlinDefaultContextCompressionPolicy))

(def default-tool-batch-object
  (marlinDefaultToolBatchPolicy))

(def default-self-evolution-object
  (marlinDefaultSelfEvolutionPolicy))

(def default-workspace-object
  (marlinDefaultWorkspacePolicy))

(def default-session-object
  (marlinDefaultSessionPolicy))

(def default-agent-object
  (marlinDefaultAgentPolicy))

(def default-hook-object
  (marlinDefaultHookSelectionPolicy))

(def default-route-object
  (marlinDefaultModelRoutePolicy))

(def default-continuation-object
  (marlinDefaultContinuationProfilePolicy))

(def default-review-object
  (marlinDefaultHumanReviewPolicy))

(def default-catalog-object
  (marlinDefaultCatalogProjectionPolicy))

(def fast-route-object
  (marlinModelRoutePolicy "fast-route" (.o route: "fast")))

(def custom-pack
  (marlinPolicyPack
   (.o id: "inventory-custom-pack"
       module: inventory-module
       policy-objects:
       (list route-object review-object hook-object evidence-object failure-object)
       object-operations:
       (list
        (marlin-add-object memory-object)
        (marlin-remove-object "hook-selection-policy" "hook")
        (marlin-disable-object "human-review-policy" "review")
        (marlin-replace-object
         "model-route-policy"
         "route"
         fast-route-object))
       allowed-hook-ids: '("hook")
       metadata: '((owner . "policy-pack-inventory-test")))))

(def custom-inventory
  (marlinPolicyPackInventory custom-pack))

;;; Boundary: Conflict receipts keep object surgery auditable instead of silent.
;; MarlinResult <- MarlinInput
(def conflict-pack
  (marlinPolicyPack
   (.o id: "inventory-conflict-pack"
       module: inventory-module
       policy-objects: (list route-object review-object)
       object-operations:
       (list
        (marlin-add-object route-object)
        (marlin-remove-object "model-route-policy" "missing-route")
        (marlin-disable-object "human-review-policy" "review")
        (marlin-disable-object "human-review-policy" "review")
        (marlin-replace-object "model-route-policy" "route" (.o invalid: "replacement")))
       allowed-hook-ids: '()
       metadata: '((owner . "policy-pack-inventory-test")))))

(def conflict-inventory
  (marlinPolicyPackInventory conflict-pack))

(def conflict-presentation
  (marlinPolicyPackPresentation conflict-pack))

(def conflict-projection
  (marlinPolicyProjection conflict-pack))

(def conflict-chain-receipt
  (marlinPolicyProjectionChainReceipt conflict-pack))

(def conflict-receipts
  (.get conflict-pack object-surgery-receipts))

(check (.get default-pack id) => "marlin-default-policy-pack")
(check (.get default-inventory kind) => marlin-policy-pack-inventory-kind)
(check (.get default-inventory policy-object-count) => 18)
(check (.get default-inventory default-policy-object-count) => 18)
(check (.get default-inventory object-operation-count) => 0)
(check (.get default-inventory allowed-hook-ids)
       => '("runtime-catalog-default-hook"))
(check (.get default-catalog kind)
       => marlin-pack-catalog-kind)
(check (.get default-catalog-presentation kind)
       => marlin-pack-catalog-presentation-kind)
(check (.get default-catalog-presentation pack-count) => 1)
(check (.get default-catalog-presentation pack-ids)
       => '("marlin-default-policy-pack"))
(check (.get default-catalog-presentation policy-object-count) => 18)
(check (.get default-catalog-presentation default-policy-object-count)
       => 18)
(check (.get default-catalog-presentation allowed-hook-ids)
       => '("runtime-catalog-default-hook"))
(check (.get default-delivery kind)
       => default-policy-delivery-receipt-kind)
(check (.get default-delivery pack-catalog-presentation-kind)
       => marlin-pack-catalog-presentation-kind)
(check (.get default-delivery pack-ids)
       => '("marlin-default-policy-pack"))
(check (.get default-delivery policy-object-count) => 18)
(check (.get default-delivery default-policy-object-count) => 18)
(check (.get default-delivery object-operation-count) => 0)
(check (.get default-delivery policy-projection-kind)
       => marlin-policy-projection-kind)
(check (.get default-delivery policy-projection-chain-receipt-kind)
       => marlin-policy-projection-chain-receipt-kind)
(check (.get default-delivery budget-receipt-kind)
       => marlin-policy-budget-receipt-kind)
(check (.get default-delivery catalog-resolution-receipt-kind)
       => marlin-policy-catalog-resolution-receipt-kind)
(check (.get default-delivery projection-receipt-family-count) => 5)
(check (.get default-delivery projection-receipt-family-ids)
       => '("module_evaluation_receipt"
            "policy_projection_receipt"
            "native_projection_payload"
            "budget_receipt"
            "catalog_resolution_receipt"))
(check (.get default-delivery module-evaluation-receipt-owner)
       => "gerbil-module-system")
(check (.get default-delivery policy-projection-receipt-owner)
       => "gerbil-poo")
(check (.get default-delivery native-projection-payload-owner)
       => "rust")
(check (.get default-delivery budget-receipt-owner)
       => "rust")
(check (.get default-delivery catalog-resolution-receipt-owner)
       => "rust")
(check (.get default-delivery catalog-resolution-allowed-hook-count)
       => 1)
(check (.get default-delivery user-entrypoints)
       => '("DefaultPolicyPack"
            "DefaultPolicyPackCatalog"
            "DefaultPolicyPackCatalogPresentation"
            "DefaultPolicyDeliveryReceipt"
            "DefaultPolicyApply"
            "DefaultPolicyProjection"))
(check (.get default-apply kind)
       => default-policy-delivery-receipt-kind)
(check (.get default-inventory policy-families)
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
            "catalog-projection-policy"))
(check (.get default-inventory policy-object-ids)
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
            "default-catalog-projection"))
(check (marlin-policy-object-family default-evidence-object)
       => "evidence-graph-policy")
(check (marlin-policy-object-id default-evidence-object)
       => "default-evidence-graph")
(check (marlin-policy-object-family default-failure-object)
       => "failure-recovery-policy")
(check (marlin-policy-object-id default-failure-object)
       => "default-failure-recovery")
(check (marlin-policy-object-family default-memory-recall-object)
       => "memory-recall-policy")
(check (marlin-policy-object-id default-memory-recall-object)
       => "default-memory-recall")
(check (marlin-policy-object-family default-memory-trigger-object)
       => "memory-trigger-policy")
(check (marlin-policy-object-id default-memory-trigger-object)
       => "default-memory-trigger")
(check (marlin-policy-object-family default-memory-retention-object)
       => "memory-retention-policy")
(check (marlin-policy-object-id default-memory-retention-object)
       => "default-memory-retention")
(check (marlin-policy-object-family default-memory-visibility-object)
       => "memory-visibility-policy")
(check (marlin-policy-object-id default-memory-visibility-object)
       => "default-memory-visibility")
(check (marlin-policy-object-family default-subagent-object)
       => "subagent-policy")
(check (marlin-policy-object-id default-subagent-object)
       => "default-subagent")
(check (marlin-policy-object-family default-context-compression-object)
       => "context-compression-policy")
(check (marlin-policy-object-id default-context-compression-object)
       => "default-context-compression")
(check (marlin-policy-object-family default-tool-batch-object)
       => "tool-batch-policy")
(check (marlin-policy-object-id default-tool-batch-object)
       => "default-tool-batch")
(check (marlin-policy-object-family default-self-evolution-object)
       => "self-evolution-policy")
(check (marlin-policy-object-id default-self-evolution-object)
       => "default-self-evolution")
(check (marlin-policy-object-family default-workspace-object)
       => "workspace-policy")
(check (marlin-policy-object-id default-workspace-object)
       => "default-workspace")
(check (marlin-policy-object-family default-session-object)
       => "session-policy")
(check (marlin-policy-object-id default-session-object)
       => "default-session")
(check (marlin-policy-object-family default-agent-object)
       => "agent-policy")
(check (marlin-policy-object-id default-agent-object)
       => "default-agent")
(check (marlin-policy-object-family default-hook-object)
       => "hook-selection-policy")
(check (marlin-policy-object-id default-hook-object)
       => "default-hook")
(check (marlin-policy-object-family default-route-object)
       => "model-route-policy")
(check (marlin-policy-object-id default-route-object)
       => "default-model-route")
(check (marlin-policy-object-family default-continuation-object)
       => "continuation-profile-policy")
(check (marlin-policy-object-id default-continuation-object)
       => "default-continuation")
(check (marlin-policy-object-family default-review-object)
       => "human-review-policy")
(check (marlin-policy-object-id default-review-object)
       => "default-human-review")
(check (marlin-policy-object-family default-catalog-object)
       => "catalog-projection-policy")
(check (marlin-policy-object-id default-catalog-object)
       => "default-catalog-projection")
(check (.get custom-inventory policy-object-ids)
       => '("fast-route" "review" "evidence" "failure" "memory"))
(check (.get custom-inventory policy-families)
       => '("model-route-policy"
            "human-review-policy"
            "evidence-graph-policy"
            "failure-recovery-policy"
            "memory-trigger-policy"))
(check (.get custom-inventory default-policy-object-ids)
       => '("route" "review" "hook" "evidence" "failure"))
(check (.get custom-inventory disabled-policy-object-ids)
       => '("review"))
(check (.get conflict-inventory policy-object-ids)
       => '("route" "review"))
(check (.get conflict-inventory disabled-policy-object-ids)
       => '("review"))
(check (.get conflict-inventory object-operation-count) => 5)
(check (.get conflict-inventory object-surgery-receipt-count) => 5)
(check (.get conflict-inventory conflict-surgery-receipt-count) => 4)
(check (.get conflict-inventory duplicate-object-conflict-count) => 1)
(check (.get conflict-inventory missing-target-conflict-count) => 1)
(check (.get conflict-inventory disabled-target-conflict-count) => 1)
(check (.get conflict-inventory invalid-replacement-conflict-count) => 1)
(check (.get conflict-presentation conflict-surgery-receipt-count) => 4)
(check (.get conflict-presentation duplicate-object-conflict-count) => 1)
(check (.get conflict-presentation missing-target-conflict-count) => 1)
(check (.get conflict-presentation disabled-target-conflict-count) => 1)
(check (.get conflict-presentation invalid-replacement-conflict-count) => 1)
(check (.get conflict-presentation projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get conflict-presentation policy-projection-receipt-kind)
       => marlin-policy-projection-kind)
(check (.get conflict-presentation policy-composition-owner)
       => "gerbil-poo")
(check (.get conflict-presentation runtime-lifecycle-owner)
       => "rust")
(check (.get conflict-projection kind)
       => marlin-policy-projection-kind)
(check (.get conflict-projection pack-id)
       => "inventory-conflict-pack")
(check (.get conflict-projection projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get conflict-projection module-system-projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get conflict-projection module-evaluation-receipt-kind)
       => (.get conflict-presentation module-evaluation-receipt-kind))
(check (.get conflict-projection policy-projection-receipt-kind)
       => marlin-policy-projection-kind)
(check (.get conflict-projection native-projection-payload-kind)
       => marlin-policy-pack-presentation-kind)
(check (.get (.get conflict-projection native-projection-payload) kind)
       => marlin-policy-pack-presentation-kind)
(check (.get conflict-projection native-projection-payload-owner)
       => "rust")
(check (.get conflict-projection budget-receipt-owner)
       => "rust")
(check (.get conflict-projection catalog-resolution-receipt-owner)
       => "rust")
(check (.get conflict-projection import-graph-owner)
       => "gerbil-module-system")
(check (.get conflict-projection option-merge-owner)
       => "gerbil-poo")
(check (.get conflict-projection extension-composition-owner)
       => "gerbil-poo")
(check (.get conflict-projection policy-composition-owner)
       => "gerbil-poo")
(check (.get conflict-projection runtime-lifecycle-owner)
       => "rust")
(check (.get conflict-projection rust-parses-scheme-source)
       => #f)
(check (.get conflict-projection rust-handler-manufactured)
       => #f)
(check (.get conflict-chain-receipt kind)
       => marlin-policy-projection-chain-receipt-kind)
(check (.get conflict-chain-receipt receipt-family-count) => 5)
(check (.get conflict-chain-receipt receipt-family-ids)
       => '("module_evaluation_receipt"
            "policy_projection_receipt"
            "native_projection_payload"
            "budget_receipt"
            "catalog_resolution_receipt"))
(check (.get conflict-chain-receipt module-evaluation-receipt-owner)
       => "gerbil-module-system")
(check (.get conflict-chain-receipt policy-projection-receipt-owner)
       => "gerbil-poo")
(check (.get conflict-chain-receipt native-projection-payload-owner)
       => "rust")
(check (.get conflict-chain-receipt budget-receipt-owner)
       => "rust")
(check (.get conflict-chain-receipt catalog-resolution-receipt-owner)
       => "rust")
(check (.get (.get conflict-chain-receipt module-evaluation-receipt) owner)
       => "gerbil-module-system")
(check (.get (.get conflict-chain-receipt policy-projection-receipt) owner)
       => "gerbil-poo")
(check (.get (.get conflict-chain-receipt native-projection-payload) owner)
       => "rust")
(check (.get (.get conflict-chain-receipt budget-receipt) owner)
       => "rust")
(check (.get (.get conflict-chain-receipt catalog-resolution-receipt) owner)
       => "rust")
(check (.get conflict-chain-receipt catalog-resolution-allowed-hook-count)
       => 0)
(check (map (lambda (receipt) (.get receipt conflict-reasons)) conflict-receipts)
       => '(("duplicate-object")
            ("missing-target")
            ()
            ("disabled-target")
            ("invalid-replacement")))
(check (map (lambda (receipt) (.get receipt valid?)) conflict-receipts)
       => '(#f #f #t #f #f))
