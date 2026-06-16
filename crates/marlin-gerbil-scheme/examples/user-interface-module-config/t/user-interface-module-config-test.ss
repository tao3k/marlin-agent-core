;;; -*- Gerbil -*-
;;; Boundary: Example test runs the user interface module configuration workflow.

(import :clan/poo/object
        :marlin/deck-runtime-modules-lib
        :marlin/deck-runtime-script
        :marlin/deck-runtime-strategy
        :marlin/deck-runtime-user-module
        :marlin/graph-loop-continuation-native-projection
        "../modules/config"
        "../modules/loop-continuation"
        "../modules/subagent"
        :std/test)

;;; Boundary: Fixture context models a downstream user command.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-context
  (.o command: "codex user-interface workflow apply"
      agent-scope: "user-interface-agent"
      workspace-root: "user-interface-module-config"))

;;; Boundary: Upstream workflow utility owns projections from user config.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-workflow
  (user-interface-module-workflow))

;;; Boundary: Public catalog entrypoint is visible to downstream users.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-catalog
  (user-interface-module-catalog))

;;; Boundary: evalModules is the public module-system evaluation entrypoint.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-eval-result
  (user-interface-module-evaluation))

;;; Boundary: Presentation receipt summarizes the complete module-system surface.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-presentation
  (user-interface-module-system-presentation))

;;; Boundary: Evaluation is projected by the upstream workflow utility.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-evaluation
  (.get user-interface-workflow evaluation))

;;; Boundary: The script is executed by real gxtest from the workspace cwd.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-result
  (marlin-deck-runtime-user-module-run-script
   user-interface-evaluation
   "user-interface-worker-script"
   user-interface-context))

;;; Boundary: Interface receipt crosses back to Rust as typed values.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-receipt
  (car
   (marlin-deck-runtime-user-module-script-interface-receipts
    user-interface-evaluation)))

;;; Boundary: Batch metrics are measured in Scheme and budgeted by Rust.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(def user-interface-metrics
  (marlin-deck-runtime-script-batch-metrics
   (marlin-deck-runtime-user-module-find-script
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
          (string=? (.get option id) option-id))
        (.get user-interface-evaluation options)))

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
(check (.get user-interface-eval-result module-count)
       => (length (.get user-interface-evaluation module-ids)))
(check (.get user-interface-eval-result extension-count)
       => (length (.get user-interface-evaluation extensions)))
(check (.get user-interface-eval-result policy-extension-object-count)
       => 1)
(check (.get user-interface-eval-result script-count)
       => (length (.get user-interface-evaluation scripts)))
(check (.get user-interface-eval-result option-count)
       => (length (.get user-interface-evaluation options)))
(check (.get user-interface-presentation kind)
       => marlin-module-system-presentation-kind)
(check (.get user-interface-presentation root-import-count) => 5)
(check (.get user-interface-presentation root-extension-count) => 1)
(check (.get user-interface-presentation root-policy-extension-object-count)
       => 1)
(check (.get user-interface-presentation projection-chain-kind)
       => marlin-module-projection-chain-kind)
(check (.get user-interface-presentation module-evaluation-receipt-kind)
       => marlin-deck-runtime-user-module-evaluation-kind)
(check (.get user-interface-presentation import-graph-owner)
       => "gerbil-module-system")
(check (.get user-interface-presentation extension-composition-owner)
       => "gerbil-poo")
(check (.get user-interface-presentation native-projection-payload-owner)
       => "rust")
(check (.get user-interface-presentation catalog-resolution-receipt-owner)
       => "rust")
(check (.get user-interface-presentation rust-parses-scheme-source)
       => #f)
(check (.get user-interface-presentation scheme-manufactures-rust-handlers)
       => #f)
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
       => '("user-interface-hook-module"
            "user-interface-base-module"
            "user-interface-script-module"
            "user-interface-loop-continuation-module"
            "user-interface-agent-module"
            "user-interface-workspace-module"
            "user-interface-root-module"))
(check (.get (user-interface-option "surface") value)
       => "downstream-user-interface")
(check (.get (user-interface-option "entry") value)
       => "interface-workflow")
(check (.get (user-interface-option "workspace-root") value)
       => "user-interface-module-config")
(check (.get (user-interface-option "interface-file") value)
       => "interface.org")
(check (.get (user-interface-option "state-file") value)
       => "state/worker-state.org")
(check (.get (user-interface-option "agent-scope") value)
       => "user-interface-agent")
(check (.get (user-interface-option "agent-class") value)
       => "customer-user-interface")
(check (.get (user-interface-option "model-profile") value)
       => "interactive")
(check (.get (user-interface-option "hook-id") value)
       => "runtime-catalog-user-interface-hook")
(check (.get (user-interface-option "hook-action") value)
       => "register")
(check (.get (user-interface-option "hook-owner") value)
       => "user-interface-worker")
(check (.get (user-interface-option "continuation-profile") value)
       => "user-interface-loop-continuation")
(check (.get (user-interface-option "layer") value)
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
