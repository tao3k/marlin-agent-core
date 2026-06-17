;;; -*- Gerbil -*-
;;; Boundary: Module owns Gerbil POO AgentGraph policy routing projections.

package: marlin

(import (only-in :clan/poo/object .get .o))

(export marlin-agent-policy-routing-native-projection-abi-id
        marlin-agent-policy-routing-native-projection-abi-version
        marlin-agent-policy-routing-projection-type-id
        marlin-agent-policy-routing-projection-schema-id
        marlin-agent-policy-routing-projection-symbol
        make-marlin-agent-policy-routing-evidence
        make-marlin-agent-policy-routing-projection
        marlin-agent-policy-routing-select-edges
        marlin-agent-policy-routing-deny
        marlin-agent-policy-routing-defer
        marlin-agent-policy-routing-project-poo-policy)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-policy-routing-native-projection-abi-id
  "marlin.agent.policy-routing.native-projection")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-policy-routing-native-projection-abi-version 1)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-policy-routing-projection-type-id
  "marlin.agent.policy-routing")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-policy-routing-projection-schema-id
  "marlin.agent.policy-routing.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-policy-routing-projection-symbol
  "marlin_agent_policy_routing_select_edges")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-agent-policy-routing-evidence kind evidence-id)
  (.o kind: kind
      evidence_id: evidence-id))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-agent-policy-routing-projection
      graph-id policy-scope root-node decision candidate-edges evidence)
  (.o type_id: marlin-agent-policy-routing-projection-type-id
      schema_id: marlin-agent-policy-routing-projection-schema-id
      graph_id: graph-id
      policy_scope: policy-scope
      root_node: root-node
      decision: decision
      candidate_edges: candidate-edges
      evidence: evidence))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-policy-routing-select-edges
      graph-id policy-scope root-node candidate-edges evidence)
  (make-marlin-agent-policy-routing-projection
   graph-id policy-scope root-node "select_edges" candidate-edges evidence))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-policy-routing-deny graph-id policy-scope root-node evidence)
  (make-marlin-agent-policy-routing-projection
   graph-id policy-scope root-node "deny" '() evidence))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-policy-routing-defer graph-id policy-scope root-node evidence)
  (make-marlin-agent-policy-routing-projection
   graph-id policy-scope root-node "defer" '() evidence))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-policy-routing-project-poo-policy
      policy graph-id root-node candidate-edges evidence)
  (marlin-agent-policy-routing-select-edges
   graph-id
   (.get policy name)
   root-node
   candidate-edges
   evidence))
