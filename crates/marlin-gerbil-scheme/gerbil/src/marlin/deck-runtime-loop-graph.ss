;;; -*- Gerbil -*-
;;; Boundary: Module owns Scheme POO builders for Rust-validated LoopGraph IR.
;;; Executors are opaque runtime catalog handles here. Scheme can compose them
;;; into topology, but only Rust can validate ownership and schedule processes.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime)

(export marlin-deck-runtime-loop-node-kind
        marlin-deck-runtime-loop-edge-kind
        marlin-deck-runtime-loop-graph-kind
        marlin-deck-runtime-compiled-loop-graph-type-id
        marlin-deck-runtime-compiled-loop-graph-schema-id
        make-marlin-deck-runtime-loop-node
        make-marlin-deck-runtime-loop-edge
        make-marlin-deck-runtime-loop-graph
        make-marlin-deck-runtime-compiled-loop-graph
        defmarlin-deck-runtime-loop-graph
        marlin-deck-runtime-loop-node-shape-valid?
        marlin-deck-runtime-loop-edge-shape-valid?
        marlin-deck-runtime-loop-graph-shape-valid?
        marlin-deck-runtime-loop-node->compiled
        marlin-deck-runtime-loop-edge->compiled
        marlin-deck-runtime-compile-loop-graph)

;;; Boundary: Source node objects stay in the Scheme control plane.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-loop-node-kind
  "marlin-deck-runtime.loop-node.v1")

;;; Boundary: Source edge objects stay in the Scheme control plane.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-loop-edge-kind
  "marlin-deck-runtime.loop-edge.v1")

;;; Boundary: Source graph objects stay in the Scheme control plane.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-loop-graph-kind
  "marlin-deck-runtime.loop-graph.v1")

;;; Boundary: Compiled graph projection is consumed by Rust validation.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-compiled-loop-graph-type-id
  "marlin.deck-runtime.compiled-loop-graph")

;;; Boundary: Schema id versions the Rust-owned LoopGraph IR projection.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-compiled-loop-graph-schema-id
  "marlin.deck-runtime.compiled-loop-graph.v1")

;;; Boundary: Node builders expose graph intent without runtime scheduling.
;;; Config stays as user-authored key/value intent so Rust can normalize it
;;; into the executor-specific map used by the runtime.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-loop-node
      node-id-value
      executor-value
      config-value)
  (.o kind: marlin-deck-runtime-loop-node-kind
      id: node-id-value
      executor: executor-value
      config: config-value))

;;; Boundary: Edge builders expose graph intent without runtime scheduling.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-loop-edge
      from-value
      to-value
      condition-value)
  (.o kind: marlin-deck-runtime-loop-edge-kind
      from: from-value
      to: to-value
      condition: condition-value))

;;; Boundary: Graph builders keep loop topology in Scheme POO objects.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-loop-graph
      graph-id-value
      node-values
      edge-values)
  (.o kind: marlin-deck-runtime-loop-graph-kind
      graph-id: graph-id-value
      nodes: node-values
      edges: edge-values))

;;; Boundary: Compiled graph keeps field names aligned with Rust IR.
;;; This object is a native projection candidate, not an execution command. The
;;; runtime must still validate node identity, edge targets, and executor ids.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-compiled-loop-graph
      graph-id-value
      node-values
      edge-values)
  (.o type_id: marlin-deck-runtime-compiled-loop-graph-type-id
      schema_id: marlin-deck-runtime-compiled-loop-graph-schema-id
      graph_id: graph-id-value
      object_system: marlin-deck-runtime-poo-package-name
      package: marlin-deck-runtime-package-name
      module: ":marlin/deck-runtime-loop-graph"
      nodes: node-values
      edges: edge-values))

;;; Boundary: Macro templates let user modules declare graph topology plainly.
;;; Expansion is intentionally just a constructor call: imports and profiles can
;;; compose graph objects as data without giving macros runtime authority.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-loop-graph ()
  ((_ binding graph-id (node ...) (edge ...))
   (def binding
     (make-marlin-deck-runtime-loop-graph
      graph-id
      (list node ...)
      (list edge ...)))))

;;; Boundary: Shape checks are lightweight. Full graph validation is Rust-owned.
;; MarlinResult <- MarlinInput
(def (loop-graph-string-active? value)
  (and value (not (string=? value ""))))

;;; Boundary: Node shape check avoids malformed POO source objects.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-loop-node-shape-valid? node)
  (and node
       (string=? (.get node kind) marlin-deck-runtime-loop-node-kind)
       (loop-graph-string-active? (.get node id))
       (loop-graph-string-active? (.get node executor))
       (list? (.get node config))))

;;; Boundary: Edge shape check avoids malformed POO source objects.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-loop-edge-shape-valid? edge)
  (and edge
       (string=? (.get edge kind) marlin-deck-runtime-loop-edge-kind)
       (loop-graph-string-active? (.get edge from))
       (loop-graph-string-active? (.get edge to))
       (or (not (.get edge condition))
           (string? (.get edge condition)))))

;;; Boundary: Graph shape check stays structural, not scheduler-semantic.
;;; Unknown edge targets and duplicate node ids stay with Rust validation so
;;; this predicate cannot become a second graph verifier with drift.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-loop-graph-shape-valid? graph)
  (and graph
       (string=? (.get graph kind) marlin-deck-runtime-loop-graph-kind)
       (loop-graph-string-active? (.get graph graph-id))
       (andmap marlin-deck-runtime-loop-node-shape-valid?
               (.get graph nodes))
       (andmap marlin-deck-runtime-loop-edge-shape-valid?
               (.get graph edges))))

;;; Boundary: Node projection produces Rust IR field names.
;;; Projection copies slots into the ABI-facing shape and does not interpret
;;; executor names. Executor ownership is resolved by the Rust runtime catalog.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-loop-node->compiled node)
  (let ((id-value (.get node id))
        (executor-value (.get node executor))
        (config-value (.get node config)))
    (.o id: id-value
        executor: executor-value
        config: config-value)))

;;; Boundary: Edge projection produces Rust IR field names.
;;; Conditions remain opaque policy labels here. Rust decides whether a runtime
;;; supports a condition and how it participates in scheduling.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-loop-edge->compiled edge)
  (let ((from-value (.get edge from))
        (to-value (.get edge to))
        (condition-value (.get edge condition)))
    (.o from: from-value
        to: to-value
        condition: condition-value)))

;;; Boundary: Scheme compiles graph intent. Rust validates before execution.
;;; This final assembly is deliberately map-only: it preserves user-authored POO
;;; topology while keeping graph identity, edge existence, and process cleanup
;;; in Rust-owned validation and observability code.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-compile-loop-graph graph)
  (make-marlin-deck-runtime-compiled-loop-graph
   (.get graph graph-id)
   (map marlin-deck-runtime-loop-node->compiled
        (.get graph nodes))
   (map marlin-deck-runtime-loop-edge->compiled
        (.get graph edges))))
