;;; -*- Gerbil -*-
;;; Boundary: Test owns the Gerbil AgentGraph policy routing projection contract.

(import :clan/poo/object
        :marlin/agent-policy-routing-native-projection
        :marlin/deck-runtime
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def routing-test-policy
  (make-marlin-deck-runtime-model-route-policy
   "gerbil.scope.agent-topology"
   "openai"
   "gpt-5.4"
   '("codex")
   '("extension-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def routing-test-evidence
  (list (make-marlin-agent-policy-routing-evidence
         "gerbil_policy_receipt"
         "gerbil.policy.receipt.1")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-agent-policy-routing-native-projection-contract)
  (let (projection
        (marlin-agent-policy-routing-project-poo-policy
         routing-test-policy
         "agent-graph.policy"
         "planner"
         '("planner-to-custom")
         routing-test-evidence))
    (check marlin-agent-policy-routing-native-projection-abi-id
           => "marlin.agent.policy-routing.native-projection")
    (check marlin-agent-policy-routing-native-projection-abi-version => 1)
    (check marlin-agent-policy-routing-projection-symbol
           => "marlin_agent_policy_routing_select_edges")
    (check (.get projection type_id)
           => marlin-agent-policy-routing-projection-type-id)
    (check (.get projection schema_id)
           => marlin-agent-policy-routing-projection-schema-id)
    (check (.get projection graph_id) => "agent-graph.policy")
    (check (.get projection policy_scope) => "gerbil.scope.agent-topology")
    (check (.get projection root_node) => "planner")
    (check (.get projection decision) => "select_edges")
    (check (.get projection candidate_edges) => '("planner-to-custom"))
    (let (evidence (car (.get projection evidence)))
      (check (.get evidence kind) => "gerbil_policy_receipt")
      (check (.get evidence evidence_id) => "gerbil.policy.receipt.1"))))

(check-agent-policy-routing-native-projection-contract)
