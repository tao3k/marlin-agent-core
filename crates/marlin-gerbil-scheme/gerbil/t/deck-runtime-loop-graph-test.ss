;;; -*- Gerbil -*-
;;; Boundary: Test owns Scheme POO LoopGraph builder and projection contracts.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-strategy
        :std/test)

;;; Boundary: Node intent is user-authored Scheme POO.
;; MarlinResult <- MarlinInput
(def loop-plan-node
  (make-marlin-deck-runtime-loop-node
   "plan"
   "scheme-policy"
   '(("profile" . "interactive"))))

;;; Boundary: Node intent may target a Rust executor without running it.
;; MarlinResult <- MarlinInput
(def loop-apply-node
  (make-marlin-deck-runtime-loop-node
   "apply"
   "rust-runtime"
   '()))

;;; Boundary: Edge intent is still Scheme policy data.
;; MarlinResult <- MarlinInput
(def loop-plan-edge
  (make-marlin-deck-runtime-loop-edge
   "plan"
   "apply"
   "policy-approved"))

;;; Boundary: Macro mirrors downstream module-style graph declarations.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-loop-graph loop-graph-fixture
  "agent-loop"
  (loop-plan-node loop-apply-node)
  (loop-plan-edge))

;;; Boundary: Invalid shape proves Scheme checks stay structural.
;; MarlinResult <- MarlinInput
(def invalid-loop-node
  (make-marlin-deck-runtime-loop-node
   ""
   "scheme-policy"
   '()))

;;; Boundary: Source objects are POO graph intent, not runtime execution.
;; MarlinResult <- MarlinInput
(def (check-loop-graph-source-shape)
  (check (.get loop-plan-node kind)
         => marlin-deck-runtime-loop-node-kind)
  (check (.get loop-plan-edge kind)
         => marlin-deck-runtime-loop-edge-kind)
  (check (.get loop-graph-fixture kind)
         => marlin-deck-runtime-loop-graph-kind)
  (check (marlin-deck-runtime-loop-node-shape-valid? loop-plan-node)
         => #t)
  (check (marlin-deck-runtime-loop-node-shape-valid? invalid-loop-node)
         => #f)
  (check (marlin-deck-runtime-loop-graph-shape-valid? loop-graph-fixture)
         => #t))

;;; Boundary: Compiled graph shape aligns with Rust CompiledLoopGraph fields.
;; MarlinResult <- MarlinInput
(def (check-compiled-loop-graph-projection)
  (let* ((compiled
          (marlin-deck-runtime-compile-loop-graph loop-graph-fixture))
         (first-node (car (.get compiled nodes)))
         (first-edge (car (.get compiled edges))))
    (check (.get compiled type_id)
           => marlin-deck-runtime-compiled-loop-graph-type-id)
    (check (.get compiled schema_id)
           => marlin-deck-runtime-compiled-loop-graph-schema-id)
    (check (.get compiled graph_id) => "agent-loop")
    (check (.get compiled object_system)
           => marlin-deck-runtime-poo-package-name)
    (check (.get compiled package)
           => marlin-deck-runtime-package-name)
    (check (.get compiled module)
           => ":marlin/deck-runtime-loop-graph")
    (check (.get first-node id) => "plan")
    (check (.get first-node executor) => "scheme-policy")
    (check (.get first-node config)
           => '(("profile" . "interactive")))
    (check (.get first-edge from) => "plan")
    (check (.get first-edge to) => "apply")
    (check (.get first-edge condition) => "policy-approved")))

(check-loop-graph-source-shape)
(check-compiled-loop-graph-projection)
