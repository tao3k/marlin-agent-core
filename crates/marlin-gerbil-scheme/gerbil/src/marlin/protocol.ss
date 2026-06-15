;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Marlin command protocol facade used by Gerbil-side adapters.

package: marlin

(import ./protocol-types)

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
