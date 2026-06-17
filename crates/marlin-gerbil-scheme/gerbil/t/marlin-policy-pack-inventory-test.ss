;;; -*- Gerbil -*-
;;; Boundary: Module tests prefab pack inventories without full runtime fixtures.

(import :clan/poo/object
        :modules/lib)

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

;;; Boundary: Object surgery keeps custom pack inventory deterministic.
;; MarlinResult <- MarlinInput
(def route-object
  (marlinPolicyObject "model-route-policy" "route" (.o route: "normal")))

(def review-object
  (marlinPolicyObject "human-review-policy" "review" (.o reviewer: "root")))

(def hook-object
  (marlinPolicyObject "hook-selection-policy" "hook" (.o hook-id: "hook")))

(def memory-object
  (marlinPolicyObject "memory-trigger-policy" "memory" (.o action: "compact")))

(def fast-route-object
  (marlinPolicyObject "model-route-policy" "fast-route" (.o route: "fast")))

(def custom-pack
  (marlinPolicyPack
   (.o id: "inventory-custom-pack"
       module: inventory-module
       policy-objects: (list route-object review-object hook-object)
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

(def conflict-receipts
  (.get conflict-pack object-surgery-receipts))

(check (.get default-pack id) => "marlin-default-policy-pack")
(check (.get default-inventory kind) => marlin-policy-pack-inventory-kind)
(check (.get default-inventory policy-object-count) => 9)
(check (.get default-inventory default-policy-object-count) => 9)
(check (.get default-inventory object-operation-count) => 0)
(check (.get default-inventory allowed-hook-ids)
       => '("runtime-catalog-default-hook"))
(check (.get default-inventory policy-families)
       => '("workspace-policy"
            "session-policy"
            "agent-policy"
            "hook-selection-policy"
            "model-route-policy"
            "continuation-profile-policy"
            "human-review-policy"
            "failure-recovery-policy"
            "catalog-projection-policy"))
(check (.get default-inventory policy-object-ids)
       => '("default-workspace"
            "default-session"
            "default-agent"
            "default-hook"
            "default-model-route"
            "default-continuation"
            "default-human-review"
            "default-failure-recovery"
            "default-catalog-projection"))
(check (.get custom-inventory policy-object-ids)
       => '("fast-route" "review" "memory"))
(check (.get custom-inventory policy-families)
       => '("model-route-policy" "human-review-policy" "memory-trigger-policy"))
(check (.get custom-inventory default-policy-object-ids)
       => '("route" "review" "hook"))
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
(check (map (lambda (receipt) (.get receipt conflict-reasons)) conflict-receipts)
       => '(("duplicate-object")
            ("missing-target")
            ()
            ("disabled-target")
            ("invalid-replacement")))
(check (map (lambda (receipt) (.get receipt valid?)) conflict-receipts)
       => '(#f #f #t #f #f))
