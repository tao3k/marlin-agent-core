;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Aggregated Gerbil runtime test entrypoint.

(import "yuanxu-policy-test"
        "yuanxu-design165-integration-test"
        "brief-to-draft-options-test"
        "envrc-test"
        "ui-workbench-test"
        "ui-helm-test"
        "ui-mygo-test"
        "task-workbench-tabs-test"
        "end-to-end-task-workflow-test"
        "context-org-contract-test"
        "context-org-boundary-test"
        "context-progressive-dispatch-test"
        "context-durable-orchestrator-test"
        "artifacts-gitops-catalog-test"
        "git-worktree-context-test"
        "context-batch-test"
        "context-causal-test"
        "deck-runtime-test"
        "deck-runtime-loop-graph-test"
        "graph-loop-continuation-abi-projection-test"
        "graph-engine-test"
        "graph-engine-c-abi-test"
        "graph-engine-governed-context-test"
        "governance-policy-test"
        "agent-policy-routing-abi-projection-test")

(export yuanxu-design165-integration-test
        brief-to-draft-options-test
        marlin-ui-helm-test
        ui-mygo-test
        task-workbench-tabs-test
        end-to-end-task-workflow-test
        context-progressive-dispatch-test
        context-durable-orchestrator-test
        artifacts-gitops-catalog-test
        git-worktree-context-test
        marlin-graph-engine-test
        marlin-graph-engine-c-abi-test
        marlin-graph-engine-governed-context-test
        marlin-governance-policy-test)
