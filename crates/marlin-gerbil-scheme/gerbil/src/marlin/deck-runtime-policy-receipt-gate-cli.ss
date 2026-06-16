;;; -*- Gerbil -*-
;;; Boundary: Scheme-owned debug entrypoint for policy substrate gate receipts.
;;; Rust may invoke this file, but it must not construct Scheme policy text.

(import (only-in :clan/poo/object .get)
        :marlin/deck-runtime-debug-policy-extension)

;;; Boundary: Facts are the stable text envelope decoded by Rust debug CLI.
;; MarlinResult <- MarlinInput
(def (emit key value)
  (display key)
  (display "\t")
  (display value)
  (newline))

;;; Boundary: Runtime parameters arrive through the process environment.
;; MarlinResult <- MarlinInput
(def policy-receipt-iterations
  (string->number (getenv "MARLIN_POLICY_RECEIPT_ITERATIONS")))

(def extension marlin-deck-runtime-debug-policy-extension)
(def policy-module marlin-deck-runtime-debug-policy-module)
(def module-catalog (marlin-deck-runtime-debug-policy-module-catalog))
(def policy-workflow (marlin-deck-runtime-debug-policy-module-workflow))
(def module-evaluation (marlin-deck-runtime-debug-policy-module-evaluation))
(def module-system-presentation
  (marlin-deck-runtime-debug-policy-module-system-presentation))
(def substrate-gate (.get policy-workflow substrate-gate))
(def catalog (marlin-deck-runtime-debug-policy-extension-catalog))

(def scheme-policy-loop-started (time->seconds (current-time)))
(def receipt
  (marlin-deck-runtime-debug-policy-extension-receipt-loop
   policy-receipt-iterations))
(def scheme-policy-loop-elapsed-micros
  (inexact->exact
   (floor
    (* 1000000
       (- (time->seconds (current-time)) scheme-policy-loop-started)))))
(def action (.get receipt dynamic-hook-action))
(def selection (.get receipt dynamic-hook-selection))

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
      (.get module-system-presentation option-merge-owner))
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
(emit "dynamic_hook_selection_selector" (.get selection selector))
