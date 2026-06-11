;;; -*- Gerbil -*-
;;; Marlin command protocol bindings used by Gerbil-side adapters.

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
        marlin-release-visibility-artifact-paths
        display-marlin-artifact
        display-marlin-compile-response
        display-gerbil-artifact-response
        display-gerbil-compile-response)

(def marlin-loop-graph-artifact-kind "LoopGraph")
(def marlin-workspace-schema-artifact-kind "WorkspaceSchema")
(def marlin-workspace-patch-intent-artifact-kind "WorkspacePatchIntent")
(def marlin-agent-scenario-contract-artifact-kind "AgentScenarioContract")
(def marlin-release-topology-artifact-kind "ReleaseTopology")
(def marlin-agent-scenario-contract-schema-id-value "marlin.agent.scenario.v1")

(def marlin-supported-artifact-kinds
  (list marlin-loop-graph-artifact-kind
        marlin-workspace-schema-artifact-kind
        marlin-workspace-patch-intent-artifact-kind
        marlin-agent-scenario-contract-artifact-kind
        marlin-release-topology-artifact-kind))

(def (marlin-supported-artifact-kind? expected)
  (if (member expected marlin-supported-artifact-kinds) #t #f))

(def (ensure-marlin-supported-artifact-kind expected)
  (unless (marlin-supported-artifact-kind? expected)
    (error "marlin gerbil protocol unsupported artifact kind"
           expected
           marlin-supported-artifact-kinds)))

(def (ensure-marlin-loop-graph-expected expected)
  (ensure-marlin-supported-artifact-kind expected))

(def (make-marlin-artifact artifact-kind artifact)
  (ensure-marlin-supported-artifact-kind artifact-kind)
  (list artifact-kind artifact))

(def (marlin-artifact-kind artifact)
  (car artifact))

(def (marlin-artifact-value artifact)
  (cadr artifact))

(def (make-marlin-loop-graph-artifact graph)
  (make-marlin-artifact marlin-loop-graph-artifact-kind graph))

(def (make-marlin-workspace-schema-artifact schema)
  (make-marlin-artifact marlin-workspace-schema-artifact-kind schema))

(def (make-marlin-workspace-patch-intent-artifact intent)
  (make-marlin-artifact marlin-workspace-patch-intent-artifact-kind intent))

(def (make-marlin-agent-scenario-contract-artifact contract)
  (make-marlin-artifact marlin-agent-scenario-contract-artifact-kind contract))

(def (make-marlin-release-topology-artifact topology)
  (make-marlin-artifact marlin-release-topology-artifact-kind topology))

(def (make-marlin-loop-node node-id executor config)
  (list node-id executor config))

(def (marlin-loop-node-id node)
  (car node))

(def (marlin-loop-node-executor node)
  (cadr node))

(def (marlin-loop-node-config node)
  (caddr node))

(def (make-marlin-loop-edge from to condition)
  (list from to condition))

(def (marlin-loop-edge-from edge)
  (car edge))

(def (marlin-loop-edge-to edge)
  (cadr edge))

(def (marlin-loop-edge-condition edge)
  (caddr edge))

(def (make-marlin-loop-graph graph-id nodes edges)
  (list graph-id nodes edges))

(def (marlin-loop-graph-id graph)
  (car graph))

(def (marlin-loop-graph-nodes graph)
  (cadr graph))

(def (marlin-loop-graph-edges graph)
  (caddr graph))

(def (make-marlin-workspace-schema schema-id required-properties todo-states)
  (list schema-id required-properties todo-states))

(def (marlin-workspace-schema-id schema)
  (car schema))

(def (marlin-workspace-schema-required-properties schema)
  (cadr schema))

(def (marlin-workspace-schema-todo-states schema)
  (caddr schema))

(def (make-marlin-workspace-patch-intent intent-id patch dry-run-first)
  (list intent-id patch dry-run-first))

(def (marlin-workspace-patch-intent-id intent)
  (car intent))

(def (marlin-workspace-patch-intent-patch intent)
  (cadr intent))

(def (marlin-workspace-patch-intent-dry-run-first intent)
  (caddr intent))

(def (make-marlin-workspace-patch reason source-agent ops)
  (list reason source-agent ops))

(def (marlin-workspace-patch-reason patch)
  (car patch))

(def (marlin-workspace-patch-source-agent patch)
  (cadr patch))

(def (marlin-workspace-patch-ops patch)
  (caddr patch))

(def (make-marlin-set-todo-op node state)
  (list 'SetTodo node state))

(def (make-marlin-set-property-op node key value)
  (list 'SetProperty node key value))

(def (make-marlin-mark-memory-candidate-op node dispatch)
  (list 'MarkMemoryCandidate node dispatch))

(def (make-marlin-agent-scenario-contract scenario)
  (list marlin-agent-scenario-contract-schema-id-value scenario))

(def (marlin-agent-scenario-contract-schema-id contract)
  (car contract))

(def (marlin-agent-scenario-contract-scenario contract)
  (cadr contract))

(def (make-marlin-agent-scenario scenario-id description steps expected-evidence)
  (list scenario-id description steps expected-evidence))

(def (marlin-agent-scenario-id scenario)
  (car scenario))

(def (marlin-agent-scenario-description scenario)
  (cadr scenario))

(def (marlin-agent-scenario-steps scenario)
  (caddr scenario))

(def (marlin-agent-scenario-expected-evidence scenario)
  (cadddr scenario))

(def (make-marlin-agent-scenario-step name input expected-event-topics expected-span-names)
  (list name input expected-event-topics expected-span-names))

(def (marlin-agent-scenario-step-name step)
  (car step))

(def (marlin-agent-scenario-step-input step)
  (cadr step))

(def (marlin-agent-scenario-step-expected-event-topics step)
  (caddr step))

(def (marlin-agent-scenario-step-expected-span-names step)
  (cadddr step))

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

(def (marlin-release-topology-id topology)
  (list-ref topology 0))

(def (marlin-release-topology-crate-name topology)
  (list-ref topology 1))

(def (marlin-release-topology-publish-enabled topology)
  (list-ref topology 2))

(def (marlin-release-topology-asset-audit-command topology)
  (list-ref topology 3))

(def (marlin-release-topology-package-assets topology)
  (list-ref topology 4))

(def (marlin-release-topology-runtime-dependency-chain topology)
  (list-ref topology 5))

(def (marlin-release-topology-workflow-dependency-chain topology)
  (list-ref topology 6))

(def (marlin-release-topology-gates topology)
  (list-ref topology 7))

(def (make-marlin-release-gate gate-id
                               command
                               requires-local-gerbil
                               required-artifacts
                               visibility)
  (list gate-id command requires-local-gerbil required-artifacts visibility))

(def (marlin-release-gate-id gate)
  (list-ref gate 0))

(def (marlin-release-gate-command gate)
  (list-ref gate 1))

(def (marlin-release-gate-requires-local-gerbil gate)
  (list-ref gate 2))

(def (marlin-release-gate-required-artifacts gate)
  (list-ref gate 3))

(def (marlin-release-gate-visibility gate)
  (list-ref gate 4))

(def (make-marlin-release-visibility report-key evidence-keys artifact-paths)
  (list report-key evidence-keys artifact-paths))

(def (marlin-release-visibility-report-key visibility)
  (list-ref visibility 0))

(def (marlin-release-visibility-evidence-keys visibility)
  (list-ref visibility 1))

(def (marlin-release-visibility-artifact-paths visibility)
  (list-ref visibility 2))

(def (display-json-string value)
  (display "\"")
  (let ((value-length (string-length value)))
    (let loop ((index 0))
      (if (< index value-length)
        (begin
          (let ((ch (string-ref value index)))
            (cond
              ((char=? ch #\") (display "\\\""))
              ((char=? ch #\\) (display "\\\\"))
              ((char=? ch #\newline) (display "\\n"))
              ((char=? ch #\tab) (display "\\t"))
              (else (display ch))))
          (loop (+ index 1)))
        #t)))
  (display "\""))

(def (display-json-nullable-string value)
  (if value
    (display-json-string value)
    (display "null")))

(def (display-json-boolean value)
  (display (if value "true" "false")))

(def (display-json-string-list values)
  (display "[")
  (let loop ((remaining values) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (display-json-string (car remaining))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-config config)
  (display "{")
  (let loop ((remaining config) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((pair (car remaining)))
          (display-json-string (car pair))
          (display ":")
          (display-json-string (cadr pair)))
        (loop (cdr remaining) #f))))
  (display "}"))

(def (display-json-nodes nodes)
  (display "[")
  (let loop ((remaining nodes) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((node (car remaining)))
          (display "{\"id\":")
          (display-json-string (marlin-loop-node-id node))
          (display ",\"executor\":")
          (display-json-string (marlin-loop-node-executor node))
          (display ",\"config\":")
          (display-json-config (marlin-loop-node-config node))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-edges edges)
  (display "[")
  (let loop ((remaining edges) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((edge (car remaining)))
          (display "{\"from\":")
          (display-json-string (marlin-loop-edge-from edge))
          (display ",\"to\":")
          (display-json-string (marlin-loop-edge-to edge))
          (display ",\"condition\":")
          (display-json-nullable-string (marlin-loop-edge-condition edge))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-loop-graph graph)
  (display "{\"graph_id\":")
  (display-json-string (marlin-loop-graph-id graph))
  (display ",\"nodes\":")
  (display-json-nodes (marlin-loop-graph-nodes graph))
  (display ",\"edges\":")
  (display-json-edges (marlin-loop-graph-edges graph))
  (display "}"))

(def (display-json-workspace-schema schema)
  (display "{\"schema_id\":")
  (display-json-string (marlin-workspace-schema-id schema))
  (display ",\"required_properties\":")
  (display-json-string-list (marlin-workspace-schema-required-properties schema))
  (display ",\"todo_states\":")
  (display-json-string-list (marlin-workspace-schema-todo-states schema))
  (display "}"))

(def (display-json-todo-state state)
  (cond
    ((or (equal? state "Todo") (equal? state "TODO") (equal? state "todo"))
     (display-json-string "Todo"))
    ((or (equal? state "Next") (equal? state "NEXT") (equal? state "next"))
     (display-json-string "Next"))
    ((or (equal? state "Wait") (equal? state "WAIT") (equal? state "wait"))
     (display-json-string "Wait"))
    ((or (equal? state "Blocked") (equal? state "BLOCKED") (equal? state "blocked"))
     (display-json-string "Blocked"))
    ((or (equal? state "Done") (equal? state "DONE") (equal? state "done"))
     (display-json-string "Done"))
    (else
     (display "{\"Custom\":")
     (display-json-string state)
     (display "}"))))

(def (display-json-workspace-patch-op op)
  (let ((op-kind (car op)))
    (cond
      ((eq? op-kind 'SetTodo)
       (display "{\"SetTodo\":{\"node\":")
       (display-json-string (cadr op))
       (display ",\"state\":")
       (display-json-todo-state (caddr op))
       (display "}}"))
      ((eq? op-kind 'SetProperty)
       (display "{\"SetProperty\":{\"node\":")
       (display-json-string (cadr op))
       (display ",\"key\":")
       (display-json-string (caddr op))
       (display ",\"value\":")
       (display-json-string (cadddr op))
       (display "}}"))
      ((eq? op-kind 'MarkMemoryCandidate)
       (display "{\"MarkMemoryCandidate\":{\"node\":")
       (display-json-string (cadr op))
       (display ",\"dispatch\":")
       (display-json-string (caddr op))
       (display "}}"))
      (else (error "marlin gerbil protocol cannot serialize workspace patch op" op-kind)))))

(def (display-json-workspace-patch-ops ops)
  (display "[")
  (let loop ((remaining ops) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (display-json-workspace-patch-op (car remaining))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-workspace-patch patch)
  (display "{\"reason\":")
  (display-json-string (marlin-workspace-patch-reason patch))
  (display ",\"source_agent\":")
  (display-json-nullable-string (marlin-workspace-patch-source-agent patch))
  (display ",\"ops\":")
  (display-json-workspace-patch-ops (marlin-workspace-patch-ops patch))
  (display "}"))

(def (display-json-workspace-patch-intent intent)
  (display "{\"intent_id\":")
  (display-json-string (marlin-workspace-patch-intent-id intent))
  (display ",\"patch\":")
  (display-json-workspace-patch (marlin-workspace-patch-intent-patch intent))
  (display ",\"dry_run_first\":")
  (display-json-boolean (marlin-workspace-patch-intent-dry-run-first intent))
  (display "}"))

(def (display-json-agent-scenario-steps steps)
  (display "[")
  (let loop ((remaining steps) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((step (car remaining)))
          (display "{\"name\":")
          (display-json-string (marlin-agent-scenario-step-name step))
          (display ",\"input\":")
          (display-json-config (marlin-agent-scenario-step-input step))
          (display ",\"expected_event_topics\":")
          (display-json-string-list
           (marlin-agent-scenario-step-expected-event-topics step))
          (display ",\"expected_span_names\":")
          (display-json-string-list
           (marlin-agent-scenario-step-expected-span-names step))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-agent-scenario scenario)
  (display "{\"id\":")
  (display-json-string (marlin-agent-scenario-id scenario))
  (display ",\"description\":")
  (display-json-nullable-string (marlin-agent-scenario-description scenario))
  (display ",\"steps\":")
  (display-json-agent-scenario-steps (marlin-agent-scenario-steps scenario))
  (display ",\"expected_evidence\":")
  (display-json-string-list (marlin-agent-scenario-expected-evidence scenario))
  (display "}"))

(def (display-json-agent-scenario-contract contract)
  (display "{\"schema_id\":")
  (display-json-string (marlin-agent-scenario-contract-schema-id contract))
  (display ",\"scenario\":")
  (display-json-agent-scenario (marlin-agent-scenario-contract-scenario contract))
  (display "}"))

(def (display-json-release-visibility visibility)
  (display "[")
  (let loop ((remaining visibility) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((entry (car remaining)))
          (display "{\"report_key\":")
          (display-json-string (marlin-release-visibility-report-key entry))
          (display ",\"evidence_keys\":")
          (display-json-string-list (marlin-release-visibility-evidence-keys entry))
          (display ",\"artifact_paths\":")
          (display-json-string-list (marlin-release-visibility-artifact-paths entry))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-release-gates gates)
  (display "[")
  (let loop ((remaining gates) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((gate (car remaining)))
          (display "{\"gate_id\":")
          (display-json-string (marlin-release-gate-id gate))
          (display ",\"command\":")
          (display-json-string (marlin-release-gate-command gate))
          (display ",\"requires_local_gerbil\":")
          (display-json-boolean (marlin-release-gate-requires-local-gerbil gate))
          (display ",\"required_artifacts\":")
          (display-json-string-list (marlin-release-gate-required-artifacts gate))
          (display ",\"visibility\":")
          (display-json-release-visibility (marlin-release-gate-visibility gate))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-release-topology topology)
  (display "{\"topology_id\":")
  (display-json-string (marlin-release-topology-id topology))
  (display ",\"crate_name\":")
  (display-json-string (marlin-release-topology-crate-name topology))
  (display ",\"publish_enabled\":")
  (display-json-boolean (marlin-release-topology-publish-enabled topology))
  (display ",\"asset_audit_command\":")
  (display-json-string (marlin-release-topology-asset-audit-command topology))
  (display ",\"package_assets\":")
  (display-json-string-list (marlin-release-topology-package-assets topology))
  (display ",\"runtime_dependency_chain\":")
  (display-json-string-list
   (marlin-release-topology-runtime-dependency-chain topology))
  (display ",\"workflow_dependency_chain\":")
  (display-json-string-list
   (marlin-release-topology-workflow-dependency-chain topology))
  (display ",\"gates\":")
  (display-json-release-gates (marlin-release-topology-gates topology))
  (display "}"))

(def (display-marlin-artifact artifact)
  (let ((artifact-kind (marlin-artifact-kind artifact))
        (artifact-value (marlin-artifact-value artifact)))
    (display "{\"artifact\":{")
    (display-json-string artifact-kind)
    (display ":")
    (cond
      ((equal? artifact-kind marlin-loop-graph-artifact-kind)
       (display-json-loop-graph artifact-value))
      ((equal? artifact-kind marlin-workspace-schema-artifact-kind)
       (display-json-workspace-schema artifact-value))
      ((equal? artifact-kind marlin-workspace-patch-intent-artifact-kind)
       (display-json-workspace-patch-intent artifact-value))
      ((equal? artifact-kind marlin-agent-scenario-contract-artifact-kind)
       (display-json-agent-scenario-contract artifact-value))
      ((equal? artifact-kind marlin-release-topology-artifact-kind)
       (display-json-release-topology artifact-value))
      (else (error "marlin gerbil protocol cannot serialize artifact kind" artifact-kind)))
    (display "}}")))

(def (display-marlin-compile-response artifact)
  (display-marlin-artifact artifact))

(def (display-gerbil-artifact-response artifact-kind artifact)
  (display-marlin-artifact (make-marlin-artifact artifact-kind artifact)))

(def (display-gerbil-compile-response graph)
  (display-marlin-compile-response (make-marlin-loop-graph-artifact graph)))
