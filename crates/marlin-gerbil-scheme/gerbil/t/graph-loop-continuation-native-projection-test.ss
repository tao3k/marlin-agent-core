;;; -*- Gerbil -*-
;;; Boundary: Test owns Gerbil POO continuation projection contract coverage.

(import :clan/poo/object
        :marlin/deck-runtime-loop-graph
        :marlin/graph-loop-continuation-native-projection
        :std/test)

;;; Boundary: Continuation graph is policy intent, not runtime execution.
;; MarlinResult <- MarlinInput
(def continuation-test-node
  (make-marlin-deck-runtime-loop-node
   "policy"
   "gerbil.poo.policy"
   '(("source" . "poo"))))

;;; Boundary: Graph source object stays in the Scheme POO plane.
;; MarlinResult <- MarlinInput
(def continuation-test-graph
  (make-marlin-deck-runtime-loop-graph
   "poo-continuation-graph"
   (list continuation-test-node)
   '()))

;;; Boundary: Compiled graph uses the Rust IR field names.
;; MarlinResult <- MarlinInput
(def continuation-test-compiled-graph
  (marlin-deck-runtime-compile-loop-graph continuation-test-graph))

;;; Boundary: Macro gives downstream policy modules a short profile form.
;; MarlinResult <- MarlinInput
(defmarlin-graph-loop-continuation-profile
  continuation-test-profile
  "customer-continuation-profile"
  (make-marlin-graph-loop-continuation-continue-with-graph-action
   continuation-test-compiled-graph)
  '("poo_continuation=continue"))

;;; Boundary: Projection fields are typed POO slots consumed by Rust.
;; MarlinResult <- MarlinInput
(def (check-graph-loop-continuation-native-projection-contract)
  (let (projection
        (marlin-graph-loop-continuation-next-action continuation-test-profile))
    (check marlin-graph-loop-continuation-native-projection-abi-id
           => "marlin.agent.gerbil-loop-continuation.native-projection")
    (check marlin-graph-loop-continuation-native-projection-abi-version => 1)
    (check marlin-graph-loop-continuation-symbol
           => "marlin_graph_loop_continuation_next_action")
    (check (.get projection type_id)
           => marlin-graph-loop-continuation-type-id)
    (check (.get projection schema_id)
           => marlin-graph-loop-continuation-schema-id)
    (check (.get (.get projection action) kind)
           => "continue_with_graph")
    (check (.get (.get (.get projection action) compiled_graph) graph_id)
           => "poo-continuation-graph")
    (check (.get (car (.get (.get (.get projection action) compiled_graph) nodes)) executor)
           => "gerbil.poo.policy")
    (check (.get projection diagnostics)
           => '("poo_continuation=continue"))))

(check-graph-loop-continuation-native-projection-contract)
