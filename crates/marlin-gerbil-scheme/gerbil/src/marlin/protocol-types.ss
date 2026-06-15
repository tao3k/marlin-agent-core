;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Marlin command protocol data constructors and accessors.

package: marlin

(export make-marlin-loop-node
        marlin-loop-graph-artifact-kind
        marlin-workspace-schema-artifact-kind
        marlin-workspace-patch-intent-artifact-kind
        marlin-agent-scenario-contract-artifact-kind
        marlin-release-topology-artifact-kind
        marlin-supported-artifact-kinds
        marlin-supported-artifact-kind?
        ensure-marlin-supported-artifact-kind
        ensure-marlin-loop-graph-expected
        make-marlin-artifact
        marlin-artifact-kind
        marlin-artifact-value
        make-marlin-loop-graph-artifact
        make-marlin-workspace-schema-artifact
        make-marlin-workspace-patch-intent-artifact
        make-marlin-agent-scenario-contract-artifact
        make-marlin-release-topology-artifact
        marlin-loop-node-id
        marlin-loop-node-executor
        marlin-loop-node-config
        make-marlin-loop-edge
        marlin-loop-edge-from
        marlin-loop-edge-to
        marlin-loop-edge-condition
        make-marlin-loop-graph
        marlin-loop-graph-id
        marlin-loop-graph-nodes
        marlin-loop-graph-edges
        make-marlin-workspace-schema
        marlin-workspace-schema-id
        marlin-workspace-schema-required-properties
        marlin-workspace-schema-todo-states
        make-marlin-workspace-patch-intent
        marlin-workspace-patch-intent-id
        marlin-workspace-patch-intent-patch
        marlin-workspace-patch-intent-dry-run-first
        make-marlin-workspace-patch
        marlin-workspace-patch-reason
        marlin-workspace-patch-source-agent
        marlin-workspace-patch-ops
        make-marlin-set-todo-op
        make-marlin-set-property-op
        make-marlin-mark-memory-candidate-op
        make-marlin-agent-scenario-contract
        marlin-agent-scenario-contract-schema-id
        marlin-agent-scenario-contract-scenario
        make-marlin-agent-scenario
        marlin-agent-scenario-id
        marlin-agent-scenario-description
        marlin-agent-scenario-steps
        marlin-agent-scenario-expected-evidence
        make-marlin-agent-scenario-step
        marlin-agent-scenario-step-name
        marlin-agent-scenario-step-input
        marlin-agent-scenario-step-expected-event-topics
        marlin-agent-scenario-step-expected-span-names
        make-marlin-release-topology
        marlin-release-topology-id
        marlin-release-topology-crate-name
        marlin-release-topology-publish-enabled
        marlin-release-topology-asset-audit-command
        marlin-release-topology-package-assets
        marlin-release-topology-runtime-dependency-chain
        marlin-release-topology-workflow-dependency-chain
        marlin-release-topology-gates
        make-marlin-release-gate
        marlin-release-gate-id
        marlin-release-gate-command
        marlin-release-gate-requires-local-gerbil
        marlin-release-gate-required-artifacts
        marlin-release-gate-visibility
        make-marlin-release-visibility
        marlin-release-visibility-report-key
        marlin-release-visibility-evidence-keys
        marlin-release-visibility-artifact-paths)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-loop-graph-artifact-kind "LoopGraph")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-workspace-schema-artifact-kind "WorkspaceSchema")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-workspace-patch-intent-artifact-kind "WorkspacePatchIntent")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-scenario-contract-artifact-kind "AgentScenarioContract")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-release-topology-artifact-kind "ReleaseTopology")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-agent-scenario-contract-schema-id-value "marlin.agent.scenario.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-supported-artifact-kinds
  (list marlin-loop-graph-artifact-kind
        marlin-workspace-schema-artifact-kind
        marlin-workspace-patch-intent-artifact-kind
        marlin-agent-scenario-contract-artifact-kind
        marlin-release-topology-artifact-kind))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-supported-artifact-kind? expected)
  (if (member expected marlin-supported-artifact-kinds) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-marlin-supported-artifact-kind expected)
  (unless (marlin-supported-artifact-kind? expected)
    (error "marlin gerbil protocol unsupported artifact kind"
           expected
           marlin-supported-artifact-kinds)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-marlin-loop-graph-expected expected)
  (ensure-marlin-supported-artifact-kind expected))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-artifact artifact-kind artifact)
  (ensure-marlin-supported-artifact-kind artifact-kind)
  (list artifact-kind artifact))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-artifact-kind artifact)
  (car artifact))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-artifact-value artifact)
  (cadr artifact))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-loop-graph-artifact graph)
  (make-marlin-artifact marlin-loop-graph-artifact-kind graph))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-workspace-schema-artifact schema)
  (make-marlin-artifact marlin-workspace-schema-artifact-kind schema))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-workspace-patch-intent-artifact intent)
  (make-marlin-artifact marlin-workspace-patch-intent-artifact-kind intent))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-agent-scenario-contract-artifact contract)
  (make-marlin-artifact marlin-agent-scenario-contract-artifact-kind contract))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-release-topology-artifact topology)
  (make-marlin-artifact marlin-release-topology-artifact-kind topology))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-loop-node node-id executor config)
  (list node-id executor config))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-node-id node)
  (car node))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-node-executor node)
  (cadr node))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-node-config node)
  (caddr node))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-loop-edge from to condition)
  (list from to condition))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-edge-from edge)
  (car edge))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-edge-to edge)
  (cadr edge))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-edge-condition edge)
  (caddr edge))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-loop-graph graph-id nodes edges)
  (list graph-id nodes edges))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-graph-id graph)
  (car graph))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-graph-nodes graph)
  (cadr graph))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-loop-graph-edges graph)
  (caddr graph))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-workspace-schema schema-id required-properties todo-states)
  (list schema-id required-properties todo-states))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-schema-id schema)
  (car schema))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-schema-required-properties schema)
  (cadr schema))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-schema-todo-states schema)
  (caddr schema))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-workspace-patch-intent intent-id patch dry-run-first)
  (list intent-id patch dry-run-first))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-patch-intent-id intent)
  (car intent))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-patch-intent-patch intent)
  (cadr intent))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-patch-intent-dry-run-first intent)
  (caddr intent))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-workspace-patch reason source-agent ops)
  (list reason source-agent ops))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-patch-reason patch)
  (car patch))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-patch-source-agent patch)
  (cadr patch))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-workspace-patch-ops patch)
  (caddr patch))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-set-todo-op node state)
  (list 'SetTodo node state))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-set-property-op node key value)
  (list 'SetProperty node key value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-mark-memory-candidate-op node dispatch)
  (list 'MarkMemoryCandidate node dispatch))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-agent-scenario-contract scenario)
  (list marlin-agent-scenario-contract-schema-id-value scenario))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-contract-schema-id contract)
  (car contract))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-contract-scenario contract)
  (cadr contract))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-agent-scenario scenario-id description steps expected-evidence)
  (list scenario-id description steps expected-evidence))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-id scenario)
  (car scenario))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-description scenario)
  (cadr scenario))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-steps scenario)
  (caddr scenario))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-expected-evidence scenario)
  (cadddr scenario))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-agent-scenario-step name input expected-event-topics expected-span-names)
  (list name input expected-event-topics expected-span-names))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-step-name step)
  (car step))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-step-input step)
  (cadr step))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-step-expected-event-topics step)
  (caddr step))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-agent-scenario-step-expected-span-names step)
  (cadddr step))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-release-topology topology-id
                                  crate-name
                                  publish-enabled
                                  asset-audit-command
                                  package-assets
                                  runtime-dependency-chain
                                  workflow-dependency-chain
                                  gates)
  (list topology-id
        crate-name
        publish-enabled
        asset-audit-command
        package-assets
        runtime-dependency-chain
        workflow-dependency-chain
        gates))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-id topology)
  (list-ref topology 0))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-crate-name topology)
  (list-ref topology 1))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-publish-enabled topology)
  (list-ref topology 2))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-asset-audit-command topology)
  (list-ref topology 3))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-package-assets topology)
  (list-ref topology 4))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-runtime-dependency-chain topology)
  (list-ref topology 5))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-workflow-dependency-chain topology)
  (list-ref topology 6))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-topology-gates topology)
  (list-ref topology 7))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-release-gate gate-id
                               command
                               requires-local-gerbil
                               required-artifacts
                               visibility)
  (list gate-id command requires-local-gerbil required-artifacts visibility))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-gate-id gate)
  (list-ref gate 0))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-gate-command gate)
  (list-ref gate 1))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-gate-requires-local-gerbil gate)
  (list-ref gate 2))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-gate-required-artifacts gate)
  (list-ref gate 3))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-gate-visibility gate)
  (list-ref gate 4))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-release-visibility report-key evidence-keys artifact-paths)
  (list report-key evidence-keys artifact-paths))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-visibility-report-key visibility)
  (list-ref visibility 0))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-visibility-evidence-keys visibility)
  (list-ref visibility 1))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-release-visibility-artifact-paths visibility)
  (list-ref visibility 2))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
