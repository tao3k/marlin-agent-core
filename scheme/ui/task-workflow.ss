;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Plan-A Task Workflow closure composing:
;;; `deck-runtime` (actor Context batch admission) +
;;; `context-system/orchestrator` (`marlin-orchestrate-task` / FunFlow CAS DAG) +
;;; `artifacts` (`DefaultArtifactCatalog` SoD + Plan-A `context.org` projection) +
;;; `ui/task-workbench` (`WorkbenchTabProjection` -> checked Marlin `UiScope`).

(import (only-in :clan/poo/object .get .o .ref .slot? object?)
        (only-in :marlin-agent-core/interface LabContext LabDurableJournal)
        (only-in :marlin-agent-core/deck-runtime/runtime open-deck-runtime)
        (only-in :marlin-agent-core/context-system/projection marlin-context-scopes->bytes)
        (only-in :marlin-agent-core/context-system/orchestrator
                 marlin-derive-worktree
                 marlin-derive-task
                 marlin-orchestrate-task
                 marlin-replan-task-epoch
                 marlin-record-task-cas!
                 marlin-commit-auto-archive!
                 marlin-reopen-task!
                 marlin-project-workbench-tabs)
        (only-in :marlin-agent-core/artifacts/objects DefaultArtifactCatalog)
        (only-in :marlin-agent-core/artifacts/funcs
                 artifacts-admit-record
                 artifacts-project-context-org
                 artifacts-inspect-context-org
                 artifacts-plan-artifact-directory
                 artifacts-materialize-artifact-directory!)
        (only-in "./task-workbench.ss"
                 make-marlin-task-workbench-package
                 marlin-task-workbench-peek-set-add
                 marlin-task-workbench-peek-set-remove))

(export marlin-task-workflow-admit-batch
        marlin-task-workflow-orchestrate-and-project
        marlin-task-workflow-commit-and-archive!)

(def (opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

;;; Admit a vector of Context Scope requests through an isolated `deck-runtime`
;;; actor, verify zero actor failures, and return `{submitted-ids digests-bytes stats}`.
(def (marlin-task-workflow-admit-batch requests
                                       actor-id: (actor-id 1)
                                       admit-fn: (admit-fn (.get LabContext admit-scope)))
  (let* ((runtime (open-deck-runtime admit-fn marlin-context-scopes->bytes))
         (total (vector-length requests)))
    (dynamic-wind
      void
      (lambda ()
        (let ((ids ((.get runtime submit-module-batch) actor-id requests)))
          (let loop ((remaining total) (chunks '()))
            (if (<= remaining 0)
              (let ((stats ((.get runtime statistics))))
                (.o submitted-ids: ids
                    digests-chunks: (reverse chunks)
                    count: total
                    stats: stats))
              (let* ((packet ((.get runtime collect) (min 32 remaining)))
                     (received (vector-ref packet 0))
                     (status (vector-ref packet 1))
                     (bytes (vector-ref packet 2))
                     (n (u64vector-length received)))
                (let check ((i 0))
                  (when (< i n)
                    (unless (zero? (u8vector-ref status i))
                      (error "Deck runtime actor rejected Context Scope request" i))
                    (check (+ i 1))))
                (unless (= (u8vector-length bytes) (* n 32))
                  (error "Deck runtime digest byte length mismatch" (u8vector-length bytes) n))
                (loop (- remaining n) (cons bytes chunks)))))))
      (lambda ()
        ((.get runtime close))))))

;;; Pure functional Plan-A workflow composition (zero filesystem/journal side effects):
;;; Org task source + governed artifact specs -> FunFlow orchestration plan +
;;; GitOps Artifact Catalog Org projection/inspection + Task-as-Tab UiPackage.
(def (marlin-task-workflow-orchestrate-and-project request)
  (let* ((worktree-name (opt-ref request 'worktree "wt/plan-a"))
         (branch-name (opt-ref request 'branch worktree-name))
         (source-cut (opt-ref request 'source-cut "HEAD"))
         (task-id (opt-ref request 'task-id "task-plan-a"))
         (task-title (opt-ref request 'title "Plan-A Task"))
         (org-source (.ref request 'source))
         (journal-state (opt-ref request 'journal-state (.o)))
         (catalog (opt-ref request 'catalog DefaultArtifactCatalog))
         (artifact-specs (opt-ref request 'artifact-specs '()))
         (extra-tasks (opt-ref request 'extra-tasks '()))
         (peek-set (opt-ref request 'peek-set '()))
         (focused-task-id (opt-ref request 'focused-task-id task-id))
         (disclosure-level (opt-ref request 'disclosure-level 'L1))
         (nerd-icons? (opt-ref request 'nerd-icons? #f))
         (wt (marlin-derive-worktree
              (.o bind-these: #f
                  worktree: worktree-name
                  branch: branch-name
                  source-cut: source-cut)))
         (task (marlin-derive-task
                (.o bind-these: #f
                    worktree-node: wt
                    task-id: task-id)))
         (orch-plan (marlin-orchestrate-task
                     (.o bind-these: #f
                         task: task
                         source: org-source
                         journal-state: journal-state)))
         (admitted-artifacts
          (map (lambda (spec) (artifacts-admit-record catalog spec))
               artifact-specs))
         (artifact-org (artifacts-project-context-org catalog admitted-artifacts))
         (artifact-inspection (artifacts-inspect-context-org catalog artifact-org))
         (artifact-dir-plan (artifacts-plan-artifact-directory catalog admitted-artifacts))
         (checklist-done (opt-ref request 'checklist-done 0))
         (checklist-total (opt-ref request 'checklist-total 1))
         (task-status (opt-ref request 'status 'in-progress))
         (archived? (opt-ref request 'archived? #f))
         (task-summary (.o bind-these: #f
                           task-id: task-id
                           title: task-title
                           status: task-status
                           archived?: archived?
                           worktree: worktree-name
                           checklist-done: checklist-done
                           checklist-total: checklist-total))
         (all-tasks (cons task-summary extra-tasks))
         (tab-projection
          (marlin-project-workbench-tabs
           (.o bind-these: #f
               owner: wt
               tasks: all-tasks
               journal-state: journal-state
               peek-set: peek-set
               focused-task-id: focused-task-id)))
         (ui-package
          (make-marlin-task-workbench-package
           tab-projection
           disclosure-level: disclosure-level
           nerd-icons?: nerd-icons?)))
    (.o bind-these: #f
        worktree: wt
        task: task
        orchestration: orch-plan
        admitted-artifacts: admitted-artifacts
        artifact-org: artifact-org
        artifact-inspection: artifact-inspection
        artifact-directory-plan: artifact-dir-plan
        tab-projection: tab-projection
        ui-package: ui-package)))

;;; Explicit effectful `!` boundary: records CAS receipt, materializes & validates
;;; the GitOps Org directory, commits atomic auto-archive into `LabDurableJournal`,
;;; and returns updated blur/peek/reopen tab projections + UiPackages.
(def (marlin-task-workflow-commit-and-archive! request)
  (let* ((journal-path (.ref request 'journal-path))
         (artifact-dir (opt-ref request 'artifact-dir #f))
         (catalog (opt-ref request 'catalog DefaultArtifactCatalog))
         (wt (.ref request 'worktree))
         (task (.ref request 'task))
         (task-id (.ref task 'task-id))
         (done-source (.ref request 'done-source))
         (index-source (opt-ref request 'index-source ""))
         (admitted-artifacts (opt-ref request 'admitted-artifacts '()))
         (cas-node-id (opt-ref request 'cas-node-id #f))
         (cas-fingerprint (opt-ref request 'cas-fingerprint #f))
         (cas-digest (opt-ref request 'cas-digest #f))
         (timestamp (opt-ref request 'timestamp "2026-11-25 Wed 00:45"))
         (materialized
          (and artifact-dir
               (artifacts-materialize-artifact-directory!
                catalog admitted-artifacts artifact-dir)))
         (journal ((.get LabDurableJournal open) journal-path)))
    (dynamic-wind
      void
      (lambda ()
        ((.get LabDurableJournal verify!) journal)
        (when (and cas-node-id cas-fingerprint cas-digest)
          (marlin-record-task-cas!
           (.o bind-these: #f
               journal: journal
               task-id: task-id
               node-id: cas-node-id
               fingerprint: cas-fingerprint
               result-digest: cas-digest)))
        (let* ((archive-receipt
                (marlin-commit-auto-archive!
                 (.o bind-these: #f
                     journal: journal
                     task: task
                     source: done-source
                     index-source: index-source
                     timestamp: timestamp)))
               (archived-summary
                (.o bind-these: #f
                    task-id: task-id
                    title: (opt-ref request 'title "Plan-A Task")
                    status: 'done
                    archived?: #t
                    worktree: (.ref wt 'worktree)
                    checklist-done: (opt-ref request 'checklist-total 2)
                    checklist-total: (opt-ref request 'checklist-total 2)))
               (blurred-tabs
                (marlin-project-workbench-tabs
                 (.o bind-these: #f
                     owner: wt
                     tasks: (list archived-summary)
                     peek-set: '()
                     focused-task-id: #f)))
               (peek-set (marlin-task-workbench-peek-set-add '() task-id))
               (peeked-tabs
                (marlin-project-workbench-tabs
                 (.o bind-these: #f
                     owner: wt
                     tasks: (list archived-summary)
                     peek-set: peek-set
                     focused-task-id: #f)))
               (reopen-receipt
                (marlin-reopen-task!
                 (.o bind-these: #f
                     journal: journal
                     task: task
                     source: done-source
                     index-source: (if (and (object? archive-receipt)
                                            (.slot? archive-receipt 'index-source))
                                     (.ref archive-receipt 'index-source)
                                     index-source)
                     timestamp: timestamp)))
               (reopened-summary
                (.o bind-these: #f
                    task-id: task-id
                    title: (opt-ref request 'title "Plan-A Task")
                    status: 'in-progress
                    archived?: #f
                    worktree: (.ref wt 'worktree)
                    checklist-done: (opt-ref request 'checklist-total 2)
                    checklist-total: (opt-ref request 'checklist-total 2)))
               (reopened-tabs
                (marlin-project-workbench-tabs
                 (.o bind-these: #f
                     owner: wt
                     tasks: (list reopened-summary)
                     peek-set: (marlin-task-workbench-peek-set-remove peek-set task-id)
                     focused-task-id: task-id)))
               (reopened-ui
                (make-marlin-task-workbench-package reopened-tabs
                                                    disclosure-level: 'L2
                                                    nerd-icons?: #f)))
          (.o bind-these: #f
              materialized: materialized
              archive-receipt: archive-receipt
              blurred-tabs: blurred-tabs
              peeked-tabs: peeked-tabs
              reopen-receipt: reopen-receipt
              reopened-tabs: reopened-tabs
              reopened-ui: reopened-ui)))
      (lambda ()
        ((.get LabDurableJournal close!) journal)))))
