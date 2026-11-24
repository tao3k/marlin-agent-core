;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Marlin declarative Durable Task Orchestrator and pure WorkbenchTabProjection.
;;; Consumes Lab FunFlow CAS DAG and Durable Orchestrator strictly through
;;; :marlin-agent-core/interface.

(import (only-in :clan/poo/object .o .ref .slot? object?)
        (only-in :marlin-agent-core/interface
                 LabFunflow LabDurableOrchestrator
                 DurableTaskPolicy. RepoNode. WorktreeNode. TaskNode.
                 DurableGovernedStrategy.
                 define-repo! define-worktree! define-durable-task!))

(export MarlinRepo.
        MarlinWorktree.
        marlin-derive-worktree
        marlin-derive-task
        marlin-orchestrate-task
        marlin-replan-task-epoch
        marlin-record-task-cas!
        marlin-recover-task
        marlin-commit-auto-archive!
        marlin-reopen-task!
        marlin-project-workbench-tabs)

(define-repo! MarlinRepo.
  repo-id: "marlin-agent-core"
  topology-path: "topology.org"
  index-path: "index.org"
  policy: (.o (:: @ DurableTaskPolicy.)
              focused-tab-archive: 'on-blur
              scratch-promotion: 'on-plan
              reopen-mode: 'peek-until-edit
              max-concurrency: 4)
  strategy: DurableGovernedStrategy.)

(define-worktree! MarlinWorktree. MarlinRepo.
  worktree: "main"
  branch: "main"
  source-cut: "HEAD")

(def (opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

(def (marlin-derive-worktree request)
  (let* ((base (opt-ref request 'repo MarlinRepo.))
         (wt (opt-ref request 'worktree "main"))
         (br (opt-ref request 'branch wt))
         (cut (opt-ref request 'source-cut "HEAD"))
         (pol (opt-ref request 'policy #f)))
    (if pol
      (.o (:: @ [base WorktreeNode.])
          worktree: wt
          branch: br
          source-cut: cut
          policy: pol)
      (.o (:: @ [base WorktreeNode.])
          worktree: wt
          branch: br
          source-cut: cut))))

(def (marlin-derive-task request)
  (let* ((wt (opt-ref request 'worktree-node MarlinWorktree.))
         (tid (opt-ref request 'task-id "task-unnamed"))
         (opath (opt-ref request 'org-path #f))
         (strat (opt-ref request 'strategy #f))
         (env (opt-ref request 'restriction-envelope #f)))
    (if strat
      (.o (:: @ [wt TaskNode.])
          task-id: tid
          org-path: opath
          strategy: strat
          restriction-envelope: env)
      (.o (:: @ [wt TaskNode.])
          task-id: tid
          org-path: opath
          restriction-envelope: env))))

(def (marlin-orchestrate-task request)
  (let ((task (.ref request 'task))
        (org-text (.ref request 'source))
        (jstate (opt-ref request 'journal-state (.o))))
    ((.ref LabDurableOrchestrator 'task-orchestration-plan) task org-text jstate)))

(def (marlin-replan-task-epoch request)
  (let* ((task (.ref request 'task))
         (prior-source (.ref request 'prior-source))
         (next-source (.ref request 'next-source))
         (completed-cas (opt-ref request 'completed-cas #f))
         (in-flight (opt-ref request 'in-flight-ids '()))
         (ctx (opt-ref request 'governance-context (.o)))
         (dag-fn (.ref LabDurableOrchestrator 'task-funflow-dag))
         (replan-fn (.ref (.ref LabFunflow 'durable-dag) 'replan-epoch))
         (prior-dag (.ref (dag-fn task prior-source) 'dag))
         (next-dag (.ref (dag-fn task next-source) 'dag)))
    (replan-fn prior-dag next-dag
               completed-cas-table: completed-cas
               in-flight-ids: in-flight
               strategy: (opt-ref task 'strategy DurableGovernedStrategy.)
               context: ctx)))

(def (marlin-record-task-cas! request)
  ((.ref LabDurableOrchestrator 'record-cas!)
   (.ref request 'journal)
   (.ref request 'task-id)
   (.ref request 'node-id)
   (.ref request 'fingerprint)
   (.ref request 'result-digest)))

(def (marlin-recover-task request)
  ((.ref LabDurableOrchestrator 'recover)
   (.ref request 'journal)
   (.ref request 'task)
   (.ref request 'source)
   in-flight-ids: (opt-ref request 'in-flight-ids '())
   unclosed-attempts: (opt-ref request 'unclosed-attempts '())
   governance-context: (opt-ref request 'governance-context (.o))))

(def (marlin-commit-auto-archive! request)
  ((.ref LabDurableOrchestrator 'commit-archive!)
   (.ref request 'journal)
   (.ref request 'task)
   (.ref request 'source)
   (opt-ref request 'index-source "")
   (opt-ref request 'journal-state (.o))
   (opt-ref request 'timestamp "2026-11-24 Tue 00:00")))

(def (marlin-reopen-task! request)
  ((.ref LabDurableOrchestrator 'reopen!)
   (.ref request 'journal)
   (.ref request 'task)
   (.ref request 'source)
   (opt-ref request 'index-source "")
   (opt-ref request 'timestamp "2026-11-24 Tue 00:00")))

(def (marlin-project-workbench-tabs request)
  ((.ref LabDurableOrchestrator 'project-workbench-tabs)
   (opt-ref request 'owner MarlinWorktree.)
   (.ref request 'tasks)
   (opt-ref request 'journal-state (.o))
   (opt-ref request 'peek-set '())
   (opt-ref request 'focused-task-id #f)))
