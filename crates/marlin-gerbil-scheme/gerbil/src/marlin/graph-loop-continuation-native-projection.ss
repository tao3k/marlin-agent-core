;;; -*- Gerbil -*-
;;; Boundary: Module owns Gerbil POO continuation projections for the Rust graph-loop controller.
;;; Responsibility: keep continuation profiles as Scheme-owned policy objects until the final
;;; native projection payload is requested by Rust.
;;; Runtime boundary: this module may import POO helpers and LoopGraph constructors, but it must
;;; not schedule graph execution, resolve Rust handlers, or serialize native runtime requests.
;;; Agent trust: downstream repairs should trust only the exported ABI ids, schema ids, profile
;;; constructors, action constructors, and `marlin-graph-loop-continuation-next-action` projection.

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

;;; Boundary: Projection is the handoff packet from Scheme policy to Rust control.
;;; Keeping action and diagnostics as POO fields lets Rust reject schema or action
;;; mismatches before graph execution without parsing Gerbil source text.
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
