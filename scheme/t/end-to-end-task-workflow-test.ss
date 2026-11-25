;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; End-to-end pure Scheme Plan-A Task Workflow integration test:
;;; `deck-runtime` actor batch admission -> `marlin-orchestrate-task` FunFlow DAG ->
;;; `artifacts` (`DefaultArtifactCatalog` SoD + `context.org` + directory Org Contract) ->
;;; `ui/task-workbench` (`WorkbenchTabProjection` + checked Marlin `UiScope`) ->
;;; durable CAS + atomic auto-archive + read-only peek + durable reopen.

(import :std/test
        (only-in :std/io/tempfile make-temporary-file-name)
        (only-in :std/list/list find)
        (only-in :clan/poo/object .get .o .ref .slot?)
        (only-in :marlin-agent-core/interface LabContext)
        (only-in "../ui/task-workflow.ss"
                 marlin-task-workflow-admit-batch
                 marlin-task-workflow-orchestrate-and-project
                 marlin-task-workflow-commit-and-archive!))

(export end-to-end-task-workflow-test)

(def sample-commit "0123456789abcdef0123456789abcdef01234567")
(def sample-digest "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
(def sample-receipt-digest "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")

(def plan-a-active-org
  (string-append
   ":PROPERTIES:\n"
   ":ORG_CONTRACT: context.session.seed.v1\n"
   ":ORG_CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":GITOPS_WORKTREE: wt/plan-a\n"
   ":END:\n"
   "* IN_PROGRESS Ship Plan-A End-to-End Task Closure [1/2]\n"
   ":PROPERTIES:\n"
   ":ID: task-plan-a-01\n"
   ":AGENT_ID: agent/worker-01\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":END:\n"
   "- [X] Wire Yuanxu Design 165 convergence\n"
   "- [ ] Close Deck + Artifacts + Workbench pipeline\n\n"
   "** DONE Wire Yuanxu Design 165 convergence\n"
   ":PROPERTIES:\n"
   ":ID: step-design165\n"
   ":AGENT_ID: agent/worker-01\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":END:\n\n"
   "** TODO Close Deck + Artifacts + Workbench pipeline\n"
   ":PROPERTIES:\n"
   ":ID: step-pipeline\n"
   ":DEPENDS_ON: step-design165\n"
   ":AGENT_ID: agent/worker-01\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":END:\n"))

(def plan-a-done-org
  (string-append
   ":PROPERTIES:\n"
   ":ORG_CONTRACT: context.session.seed.v1\n"
   ":ORG_CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":GITOPS_WORKTREE: wt/plan-a\n"
   ":END:\n"
   "* DONE Ship Plan-A End-to-End Task Closure [2/2]\n"
   ":PROPERTIES:\n"
   ":ID: task-plan-a-01\n"
   ":AGENT_ID: agent/worker-01\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":END:\n"
   "- [X] Wire Yuanxu Design 165 convergence\n"
   "- [X] Close Deck + Artifacts + Workbench pipeline\n\n"
   "** DONE Wire Yuanxu Design 165 convergence\n"
   ":PROPERTIES:\n"
   ":ID: step-design165\n"
   ":AGENT_ID: agent/worker-01\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":END:\n\n"
   "** DONE Close Deck + Artifacts + Workbench pipeline\n"
   ":PROPERTIES:\n"
   ":ID: step-pipeline\n"
   ":DEPENDS_ON: step-design165\n"
   ":AGENT_ID: agent/worker-01\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":END:\n"))

(def (make-scope-request turn-idx)
  (.o (:: @ (.get LabContext scope-prototype))
      bundle: "plan-a"
      organization: "marlin-engineering-org"
      epoch: 1
      project: "marlin-agent-core"
      worktree: "wt/plan-a"
      source-cut: sample-commit
      actor: "agent/worker-01"
      task: "task-plan-a-01"
      destination: "workbench"
      session: "plan-a-e2e"
      turn: turn-idx))

(def end-to-end-task-workflow-test
  (test-suite "Marlin Plan-A End-to-End Task Workflow (Deck + Artifacts + Orchestrator + Workbench)"

    (test-case "Deck runtime actor batch admits typed Context Scope requests into 32-byte digests"
      (let* ((requests (vector (make-scope-request 0)
                               (make-scope-request 1)
                               (make-scope-request 2)))
             (batch (marlin-task-workflow-admit-batch requests)))
        (check (.ref batch 'count) => 3)
        (check (u64vector-length (.ref batch 'submitted-ids)) => 3)
        (check (apply + (map u8vector-length (.ref batch 'digests-chunks))) => (* 3 32))))

    (test-case "Pure Plan-A orchestration projects governed Artifacts, Plan-A Org, and checked Task Workbench UI"
      (let* ((patch-spec (.o bind-these: #f
                             artifact-kind: 'git-patch
                             collection: 'patches
                             commit: sample-commit
                             worktree: "wt/plan-a"
                             digest: sample-digest
                             producer: 'worker
                             reviewer: 'verifier))
             (receipt-spec (.o bind-these: #f
                               artifact-kind: 'test-receipt
                               collection: 'receipts
                               commit: sample-commit
                               worktree: "wt/plan-a"
                               digest: sample-receipt-digest
                               producer: 'verifier))
             (result
              (marlin-task-workflow-orchestrate-and-project
               (.o bind-these: #f
                   worktree: "wt/plan-a"
                   task-id: "task-plan-a-01"
                   title: "Ship Plan-A Closure"
                   source: plan-a-active-org
                   artifact-specs: (list patch-spec receipt-spec)
                   checklist-done: 1
                   checklist-total: 2
                   status: 'in-progress
                   disclosure-level: 'L2)))
             (inspected (.ref result 'artifact-inspection))
             (tabs (.ref result 'tab-projection))
             (ui-pkg (.ref result 'ui-package)))
        (check (.get inspected valid?) => #t)
        (check (length (.get inspected artifacts)) => 2)
        (check (.ref tabs 'visible-tab-ids) => '(index "task-plan-a-01"))
        (check (and (.slot? ui-pkg 'workbench) #t) => #t)
        ;; Full durable `!` closure: materialize & validate GitOps Org directory,
        ;; record CAS receipt, auto-archive -> blur hides -> peek read-only -> reopen restores!
        (let* ((tmp-journal (make-temporary-file-name "marlin-plan-a-journal"))
               (tmp-dir (make-temporary-file-name "marlin-plan-a-artifacts"))
               (durable-res
                (marlin-task-workflow-commit-and-archive!
                 (.o bind-these: #f
                     journal-path: tmp-journal
                     artifact-dir: tmp-dir
                     worktree: (.ref result 'worktree)
                     task: (.ref result 'task)
                     title: "Ship Plan-A Closure"
                     done-source: plan-a-done-org
                     admitted-artifacts: (.ref result 'admitted-artifacts)
                     cas-node-id: "step-pipeline"
                     cas-fingerprint: "fp-plan-a-pipeline"
                     cas-digest: sample-receipt-digest
                     checklist-total: 2))))
          (let ((validation (.get (.ref durable-res 'materialized) validation)))
            (check (.get validation complete?) => #t)
            (check (.get validation passed?) => #t))
          ;; After archive + blur, only pinned `index` remains visible
          (check (.ref (.ref durable-res 'blurred-tabs) 'visible-tab-ids) => '(index))
          ;; Peek mounts archived task read-only
          (check (.ref (.ref durable-res 'peeked-tabs) 'visible-tab-ids)
                 => '(index "task-plan-a-01"))
          (let ((peek-tab (find (lambda (t) (equal? (.ref t 'id) "task-plan-a-01"))
                                (.ref (.ref durable-res 'peeked-tabs) 'dynamic-tabs))))
            (check (.ref peek-tab 'mode) => 'peek-readonly))
          ;; Reopen restores active task tab + checked UI package
          (check (.ref (.ref durable-res 'reopened-tabs) 'visible-tab-ids)
                 => '(index "task-plan-a-01"))
          (check (and (.slot? (.ref durable-res 'reopened-ui) 'workbench) #t) => #t))))))
