;;; -*- Gerbil -*-
;;; Boundary: Scheme-owned debug entrypoint for policy substrate gate receipts.
;;; Rust may invoke this file, but it must not construct Scheme policy text.

(import (only-in :clan/poo/object .get)
        :marlin/deck-runtime-debug-policy-extension
        :marlin/modules/prefabs/default-policy)

(export emit-policy-receipt-gate-cli-report)

;;; Boundary: Facts are the stable text envelope decoded by Rust debug CLI.
;; MarlinResult <- MarlinInput
(def (emit key value)
  (display key)
  (display "\t")
  (display value)
  (newline))

;;; Boundary: List facts stay simple TSV values for Rust debug parsing.
;; MarlinResult <- MarlinInput
(def (string-list->csv values)
  (foldl (lambda (value result)
           (if (string=? result "")
             value
             (string-append result "," value)))
         ""
         values))

;;; Boundary: Rust receives scalar fact values, not Scheme list syntax.
;; MarlinResult <- MarlinInput
(def (emit-string-list key values)
  (emit key (string-list->csv values)))

;;; Boundary: Runtime parameters arrive through the process environment.
;; MarlinResult <- MarlinInput
(def policy-receipt-iterations
  (string->number (getenv "MARLIN_POLICY_RECEIPT_ITERATIONS")))

;;; Boundary: Debug extension is imported as a typed POO value.
;; MarlinResult <- MarlinInput
(def extension marlin-deck-runtime-debug-policy-extension)

;;; Boundary: Policy module evidence stays Scheme-owned until projection.
;; MarlinResult <- MarlinInput
(def policy-module marlin-deck-runtime-debug-policy-module)

;;; Boundary: Module catalog is a Scheme value, not a Rust source parser result.
;; MarlinResult <- MarlinInput
(def module-catalog (marlin-deck-runtime-debug-policy-module-catalog))

;;; Boundary: Policy workflow owns the substrate gate receipt.
;; MarlinResult <- MarlinInput
(def policy-workflow (marlin-deck-runtime-debug-policy-module-workflow))

;;; Boundary: Module evaluation is emitted as typed receipt facts.
;; MarlinResult <- MarlinInput
(def module-evaluation (marlin-deck-runtime-debug-policy-module-evaluation))

;;; Boundary: Module presentation is the Rust-readable scalar view.
;; MarlinResult <- MarlinInput
(def module-system-presentation
  (marlin-deck-runtime-debug-policy-module-system-presentation))

;;; Boundary: Policy pack presentation exposes prefab object surgery receipts.
;; MarlinResult <- MarlinInput
(def policy-pack-presentation
  (marlin-deck-runtime-debug-policy-pack-presentation))

;;; Boundary: Policy projection fixes the Scheme -> Rust handoff envelope.
;; MarlinResult <- MarlinInput
(def policy-projection
  (marlin-deck-runtime-debug-policy-projection))

;;; Boundary: Projection chain receipt exposes each typed handoff stage.
;; MarlinResult <- MarlinInput
(def policy-projection-chain-receipt
  (marlin-deck-runtime-debug-policy-projection-chain-receipt))

;;; Boundary: Shared default prefab pack is also visible to debug CLI facts.
;; MarlinResult <- MarlinInput
(def default-policy-delivery
  (DefaultPolicyDeliveryReceipt policy-module))

;;; Boundary: Default policy projection proves the shared prefab handoff.
;; MarlinResult <- MarlinInput
(def default-policy-projection
  (DefaultPolicyProjection policy-module))

;;; Boundary: Substrate gate proves policy evaluation before Rust runtime use.
;; MarlinResult <- MarlinInput
(def substrate-gate (.get policy-workflow substrate-gate))

;;; Boundary: Runtime catalog stays Scheme-selected but Rust-owned by id.
;; MarlinResult <- MarlinInput
(def catalog (marlin-deck-runtime-debug-policy-extension-catalog))

;;; Boundary: Timing starts after module construction to isolate policy looping.
;; MarlinResult <- MarlinInput
(def scheme-policy-loop-started (time->seconds (current-time)))

;;; Boundary: Receipt loop returns the last typed policy receipt.
;; MarlinResult <- MarlinInput
(def receipt
  (marlin-deck-runtime-debug-policy-extension-receipt-loop
   policy-receipt-iterations))

;;; Boundary: Elapsed micros are scalar debug facts for Rust parsing.
;; MarlinResult <- MarlinInput
(def scheme-policy-loop-elapsed-micros
  (inexact->exact
   (floor
    (* 1000000
       (- (time->seconds (current-time)) scheme-policy-loop-started)))))

;;; Boundary: Dynamic hook action is decoded from the typed Scheme receipt.
;; MarlinResult <- MarlinInput
(def action (.get receipt dynamic-hook-action))

;;; Boundary: Selection evidence records how Scheme policy matched.
;; MarlinResult <- MarlinInput
(def selection (.get receipt dynamic-hook-selection))

;;; Boundary: keep executable CLI output behind a named entrypoint so the
;;; harness can analyze this file as a module without treating every field
;;; emission as independent top-level execution.
;; Void <- Void
(def (emit-policy-receipt-gate-cli-report)
  (emit "extension_kind" (.get extension kind))
(emit "extension_id" (.get extension id))
(emit "extension_source" marlin-deck-runtime-debug-policy-extension-source)
(emit "extension_surface" "poo-extension-object")
(emit "extension_capability_count" (length (.get extension capabilities)))
(emit "policy_extension_object_kind" (.get extension policy-extension-kind))
(emit "policy_extension_object" (.get extension policy-extension-object))
(emit "policy_extension_source" (.get extension policy-extension-source))
(emit "policy_extension_managed_by" (.get extension policy-extension-managed-by))
(emit "policy_extension_projection_owner"
      (.get extension policy-extension-projection-owner))
(emit "policy_extension_runtime_owner"
      (.get extension policy-extension-runtime-owner))
(emit "policy_module_kind" (.get policy-module kind))
(emit "policy_module_id" (.get policy-module id))
(emit "policy_module_family" (.get policy-module policy-family))
(emit "policy_projection_target" (.get policy-module projection-target))
(emit "module_catalog_kind" (.get module-catalog kind))
(emit "module_catalog_count" (length (.get module-catalog modules)))
(emit "module_eval_result_kind" (.get module-evaluation kind))
(emit "module_eval_workflow_kind" (.get module-evaluation workflow-kind))
(emit "module_system_presentation_kind"
      (.get module-system-presentation kind))
(emit "module_system_projection_chain_kind"
      (.get module-system-presentation projection-chain-kind))
(emit "module_system_root_import_count"
      (.get module-system-presentation root-import-count))
(emit "module_system_root_extension_count"
      (.get module-system-presentation root-extension-count))
(emit "module_system_root_policy_extension_object_count"
      (.get module-system-presentation root-policy-extension-object-count))
(emit "module_system_import_graph_owner"
      (.get module-system-presentation import-graph-owner))
(emit "module_system_option_merge_owner"
      (.get module-system-presentation option-policy-owner))
(emit "module_system_extension_composition_owner"
      (.get module-system-presentation extension-composition-owner))
(emit "module_system_native_projection_payload_owner"
      (.get module-system-presentation native-projection-payload-owner))
(emit "module_system_budget_receipt_owner"
      (.get module-system-presentation budget-receipt-owner))
(emit "module_system_catalog_resolution_receipt_owner"
      (.get module-system-presentation catalog-resolution-receipt-owner))
(emit "module_system_rust_parses_scheme_source"
      (.get module-system-presentation rust-parses-scheme-source))
(emit "module_system_scheme_manufactures_rust_handlers"
      (.get module-system-presentation scheme-manufactures-rust-handlers))
(emit "policy_pack_kind" (.get policy-pack-presentation pack-kind))
(emit "policy_pack_id" (.get policy-pack-presentation pack-id))
(emit "policy_pack_presentation_kind" (.get policy-pack-presentation kind))
(emit "policy_pack_inventory_kind"
      (.get policy-pack-presentation policy-pack-inventory-kind))
(emit "policy_pack_module_system_presentation_kind"
      (.get policy-pack-presentation module-system-presentation-kind))
(emit "policy_pack_object_count"
      (.get policy-pack-presentation policy-object-count))
(emit "policy_pack_default_object_count"
      (.get policy-pack-presentation default-policy-object-count))
(emit "policy_pack_disabled_object_count"
      (.get policy-pack-presentation disabled-policy-object-count))
(emit-string-list "policy_pack_policy_families"
                  (.get policy-pack-presentation policy-families))
(emit-string-list "policy_pack_policy_object_ids"
                  (.get policy-pack-presentation policy-object-ids))
(emit-string-list "policy_pack_default_policy_object_ids"
                  (.get policy-pack-presentation default-policy-object-ids))
(emit-string-list "policy_pack_disabled_policy_object_ids"
                  (.get policy-pack-presentation disabled-policy-object-ids))
(emit "policy_pack_operation_count"
      (.get policy-pack-presentation object-operation-count))
(emit "policy_pack_surgery_receipt_count"
      (.get policy-pack-presentation object-surgery-receipt-count))
(emit "policy_pack_conflict_surgery_receipt_count"
      (.get policy-pack-presentation conflict-surgery-receipt-count))
(emit "policy_pack_duplicate_object_conflict_count"
      (.get policy-pack-presentation duplicate-object-conflict-count))
(emit "policy_pack_missing_target_conflict_count"
      (.get policy-pack-presentation missing-target-conflict-count))
(emit "policy_pack_disabled_target_conflict_count"
      (.get policy-pack-presentation disabled-target-conflict-count))
(emit "policy_pack_invalid_replacement_conflict_count"
      (.get policy-pack-presentation invalid-replacement-conflict-count))
(emit "policy_pack_add_count"
      (.get policy-pack-presentation add-operation-count))
(emit "policy_pack_remove_count"
      (.get policy-pack-presentation remove-operation-count))
(emit "policy_pack_disable_count"
      (.get policy-pack-presentation disable-operation-count))
(emit "policy_pack_replace_count"
      (.get policy-pack-presentation replace-operation-count))
(emit "policy_pack_matched_surgery_receipt_count"
      (.get policy-pack-presentation matched-surgery-receipt-count))
(emit "policy_pack_allowed_hook_count"
      (.get policy-pack-presentation allowed-hook-count))
(emit-string-list "policy_pack_allowed_hook_ids"
                  (.get policy-pack-presentation allowed-hook-ids))
(emit "policy_pack_import_graph_owner"
      (.get policy-pack-presentation import-graph-owner))
(emit "policy_pack_option_merge_owner"
      (.get policy-pack-presentation option-merge-owner))
(emit "policy_pack_extension_composition_owner"
      (.get policy-pack-presentation extension-composition-owner))
(emit "policy_pack_native_projection_payload_owner"
      (.get policy-pack-presentation native-projection-payload-owner))
(emit "policy_pack_rust_parses_scheme_source"
      (.get policy-pack-presentation rust-parses-scheme-source))
(emit "policy_pack_rust_handler_manufactured"
      (.get policy-pack-presentation rust-handler-manufactured))
(emit "policy_projection_kind" (.get policy-projection kind))
(emit "policy_projection_pack_id" (.get policy-projection pack-id))
(emit "policy_projection_chain_kind"
      (.get policy-projection projection-chain-kind))
(emit "policy_projection_module_evaluation_receipt_kind"
      (.get policy-projection module-evaluation-receipt-kind))
(emit "policy_projection_policy_projection_receipt_kind"
      (.get policy-projection policy-projection-receipt-kind))
(emit "policy_projection_native_projection_payload_kind"
      (.get policy-projection native-projection-payload-kind))
(emit "policy_projection_native_projection_payload_owner"
      (.get policy-projection native-projection-payload-owner))
(emit "policy_projection_budget_receipt_owner"
      (.get policy-projection budget-receipt-owner))
(emit "policy_projection_catalog_resolution_receipt_owner"
      (.get policy-projection catalog-resolution-receipt-owner))
(emit "policy_projection_import_graph_owner"
      (.get policy-projection import-graph-owner))
(emit "policy_projection_option_merge_owner"
      (.get policy-projection option-merge-owner))
(emit "policy_projection_extension_composition_owner"
      (.get policy-projection extension-composition-owner))
(emit "policy_projection_policy_composition_owner"
      (.get policy-projection policy-composition-owner))
(emit "policy_projection_runtime_lifecycle_owner"
      (.get policy-projection runtime-lifecycle-owner))
(emit "policy_projection_rust_parses_scheme_source"
      (.get policy-projection rust-parses-scheme-source))
(emit "policy_projection_rust_handler_manufactured"
      (.get policy-projection rust-handler-manufactured))
(emit "policy_projection_replayable"
      (.get policy-projection replayable))
(emit "policy_projection_chain_receipt_kind"
      (.get policy-projection-chain-receipt kind))
(emit "policy_projection_chain_receipt_pack_id"
      (.get policy-projection-chain-receipt pack-id))
(emit "policy_projection_chain_module_evaluation_receipt_kind"
      "marlin.modules.policy-pack.module-evaluation-receipt.v1")
(emit "policy_projection_chain_policy_projection_receipt_kind"
      "marlin.modules.policy-projection.v1")
(emit "policy_projection_chain_native_projection_payload_kind"
      "marlin.modules.policy-pack-presentation.v1")
(emit "policy_projection_chain_budget_receipt_kind"
      "marlin.runtime.policy-budget-receipt.v1")
(emit "policy_projection_chain_catalog_resolution_receipt_kind"
      "marlin.runtime.policy-catalog-resolution-receipt.v1")
(emit "policy_projection_chain_receipt_family_count"
      (.get policy-projection-chain-receipt receipt-family-count))
(emit-string-list "policy_projection_chain_receipt_family_ids"
                  '("module_evaluation_receipt"
                    "policy_projection_receipt"
                    "native_projection_payload"
                    "budget_receipt"
                    "catalog_resolution_receipt"))
(emit "policy_projection_chain_module_evaluation_receipt_owner"
      (.get policy-projection-chain-receipt
            module-evaluation-receipt-owner))
(emit "policy_projection_chain_policy_projection_receipt_owner"
      (.get policy-projection-chain-receipt
            policy-projection-receipt-owner))
(emit "policy_projection_chain_native_projection_payload_owner"
      (.get policy-projection-chain-receipt
            native-projection-payload-owner))
(emit "policy_projection_chain_budget_receipt_owner"
      (.get policy-projection-chain-receipt budget-receipt-owner))
(emit "policy_projection_chain_catalog_resolution_receipt_owner"
      (.get policy-projection-chain-receipt
            catalog-resolution-receipt-owner))
(emit "policy_projection_chain_catalog_allowed_hook_count"
      (.get policy-pack-presentation allowed-hook-count))
(emit "policy_projection_chain_replayable"
      (.get policy-projection-chain-receipt replayable))
(emit "default_policy_delivery_kind" (.get default-policy-delivery kind))
(emit "default_policy_pack_id" (.get default-policy-delivery pack-id))
(emit "default_policy_pack_count" (.get default-policy-delivery pack-count))
(emit-string-list "default_policy_pack_ids"
                  (.get default-policy-delivery pack-ids))
(emit "default_policy_object_count"
      (.get default-policy-delivery policy-object-count))
(emit "default_policy_default_object_count"
      (.get default-policy-delivery default-policy-object-count))
(emit "default_policy_allowed_hook_count"
      (length (.get default-policy-delivery allowed-hook-ids)))
(emit-string-list "default_policy_allowed_hook_ids"
                  (.get default-policy-delivery allowed-hook-ids))
(emit "default_policy_catalog_presentation_kind"
      (.get default-policy-delivery pack-catalog-presentation-kind))
(emit "default_policy_projection_kind"
      (.get default-policy-projection kind))
(emit "default_policy_projection_chain_receipt_kind"
      (.get default-policy-delivery policy-projection-chain-receipt-kind))
(emit "default_policy_budget_receipt_kind"
      (.get default-policy-delivery budget-receipt-kind))
(emit "default_policy_catalog_resolution_receipt_kind"
      (.get default-policy-delivery catalog-resolution-receipt-kind))
(emit "default_policy_replayable"
      (.get default-policy-delivery replayable))
(emit "policy_substrate_gate_kind" (.get substrate-gate kind))
(emit "policy_substrate_gate_profile" (.get substrate-gate gate-profile))
(emit "policy_substrate_gate_receipt_kind" (.get substrate-gate receipt-kind))
(emit "policy_module_evaluation_kind" (.get substrate-gate module-evaluation-kind))
(emit "policy_module_count" (.get substrate-gate module-count))
(emit "policy_extension_count" (.get substrate-gate extension-count))
(emit "policy_extension_object_count"
      (.get substrate-gate policy-extension-object-count))
(emit "policy_script_count" (.get substrate-gate script-count))
(emit "policy_option_count" (.get substrate-gate option-count))
(emit "policy_validation_receipt_count"
      (.get substrate-gate validation-receipt-count))
(emit "policy_substrate_gate_replayable" (.get substrate-gate replayable))
(emit "scheme_policy_owner" (.get substrate-gate scheme-policy-owner))
(emit "rust_kernel_owner" (.get substrate-gate rust-kernel-owner))
(emit "catalog_kind" (.get catalog kind))
(emit "scheme_catalog_role" "extension-object-selection")
(emit "runtime_catalog_owner" "rust")
(emit "catalog_resolved_by_scheme" #f)
(emit "iterations" policy-receipt-iterations)
(emit "scheme_policy_loop_elapsed_micros" scheme-policy-loop-elapsed-micros)
(emit "avg_scheme_policy_micros_per_iteration"
      (quotient scheme-policy-loop-elapsed-micros policy-receipt-iterations))
(emit "receipt_kind" (.get receipt kind))
(emit "matched" (.get receipt matched))
(emit "policy_engine" (.get receipt policy-engine))
(emit "extension_receipt_id" (.get receipt extension-id))
(emit "dynamic_hook_action" (.get action action))
(emit "dynamic_hook_hook_id" (.get action hook-id))
(emit "dynamic_hook_registration" (.get action registration))
(emit "dynamic_hook_selection_source" (.get selection source))
(emit "dynamic_hook_selection_selector" (.get selection selector)))
