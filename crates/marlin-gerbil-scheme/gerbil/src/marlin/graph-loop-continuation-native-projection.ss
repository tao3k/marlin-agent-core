;;; -*- Gerbil -*-
;;; Boundary: Module owns Gerbil POO continuation projections for the Rust graph-loop controller.
;;; Scheme chooses policy data; Rust validates and schedules the resulting loop action.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime
        :marlin/deck-runtime-loop-graph)

(export marlin-graph-loop-continuation-native-projection-abi-id
        marlin-graph-loop-continuation-native-projection-abi-version
        marlin-graph-loop-continuation-type-id
        marlin-graph-loop-continuation-schema-id
        marlin-graph-loop-continuation-symbol
        make-marlin-graph-loop-continuation-profile
        defmarlin-graph-loop-continuation-profile
        make-marlin-graph-loop-continuation-stop-completed-action
        make-marlin-graph-loop-continuation-stop-failed-action
        make-marlin-graph-loop-continuation-escalate-to-human-action
        make-marlin-graph-loop-continuation-continue-with-graph-action
        make-marlin-graph-loop-continuation-projection
        marlin-graph-loop-continuation-next-action)

;;; Boundary: ABI id is mirrored by Rust readiness contracts.
;; MarlinResult <- MarlinInput
(def marlin-graph-loop-continuation-native-projection-abi-id
  "marlin.agent.gerbil-loop-continuation.native-projection")

;;; Boundary: ABI version is Rust-owned and checked before action compilation.
;; MarlinResult <- MarlinInput
(def marlin-graph-loop-continuation-native-projection-abi-version 1)

;;; Boundary: Type id matches the Rust projection manifest.
;; MarlinResult <- MarlinInput
(def marlin-graph-loop-continuation-type-id
  "marlin.agent.gerbil-loop-continuation")

;;; Boundary: Schema id versions the typed continuation request.
;; MarlinResult <- MarlinInput
(def marlin-graph-loop-continuation-schema-id
  "marlin.agent.gerbil_loop_graph_continuation.v1")

;;; Boundary: Native symbol is data here; Rust owns native binding resolution.
;; MarlinResult <- MarlinInput
(def marlin-graph-loop-continuation-symbol
  "marlin_graph_loop_continuation_next_action")

;;; Boundary: Profiles are user-extensible POO objects, not serialized protocol blobs.
;; MarlinResult <- MarlinInput
(def (make-marlin-graph-loop-continuation-profile
      profile-id
      action-value
      diagnostics-value)
  (.o id: profile-id
      package: marlin-deck-runtime-package-name
      module: ":marlin/graph-loop-continuation-native-projection"
      object_system: marlin-deck-runtime-poo-package-name
      action: action-value
      diagnostics: diagnostics-value))

;;; Boundary: Macro gives downstream modules a concise continuation profile form.
;; MarlinResult <- MarlinInput
(defrules defmarlin-graph-loop-continuation-profile ()
  ((_ binding profile-id action diagnostics)
   (def binding
     (make-marlin-graph-loop-continuation-profile
      profile-id
      action
      diagnostics))))

;;; Boundary: Terminal actions are typed POO data for Rust controller compilation.
;; MarlinResult <- MarlinInput
(def (make-marlin-graph-loop-continuation-stop-completed-action)
  (.o kind: "stop_completed"))

;;; Boundary: Terminal actions are typed POO data for Rust controller compilation.
;; MarlinResult <- MarlinInput
(def (make-marlin-graph-loop-continuation-stop-failed-action)
  (.o kind: "stop_failed"))

;;; Boundary: Escalation reason stays explicit data for Rust controller receipts.
;; MarlinResult <- MarlinInput
(def (make-marlin-graph-loop-continuation-escalate-to-human-action reason-value)
  (.o kind: "escalate_to_human"
      reason: reason-value))

;;; Boundary: Compiled graph comes from the Gerbil LoopGraph POO compiler.
;;; Rust still validates graph identity, edge targets, executor ids, and limits.
;; MarlinResult <- MarlinInput
(def (make-marlin-graph-loop-continuation-continue-with-graph-action
      compiled-graph-value)
  (.o kind: "continue_with_graph"
      compiled_graph: compiled-graph-value))

;;; Boundary: Projection shape mirrors Rust GerbilLoopGraphContinuationRequest.
;; MarlinResult <- MarlinInput
(def (make-marlin-graph-loop-continuation-projection
      action-value
      diagnostics-value)
  (.o type_id: marlin-graph-loop-continuation-type-id
      schema_id: marlin-graph-loop-continuation-schema-id
      action: action-value
      diagnostics: diagnostics-value))

;;; Boundary: Public projection entrypoint is policy-only; Rust owns execution.
;; MarlinResult <- MarlinInput
(def (marlin-graph-loop-continuation-next-action profile)
  (make-marlin-graph-loop-continuation-projection
   (.get profile action)
   (.get profile diagnostics)))
