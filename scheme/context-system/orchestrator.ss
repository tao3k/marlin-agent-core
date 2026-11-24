;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Marlin `+durable` Feature implementation for `context-system`:
;;; Consumes Lab Pure Durable Work Engine (`LabDurableWork`) and
;;; Context `+durable` Feature (`LabContextDurable`) strictly through
;;; `:marlin-agent-core/interface`.

(import (only-in :clan/poo/object .def .o .ref .slot? object?)
        (only-in :marlin-agent-core/interface
                 LabFunflow LabDurableWork LabContextDurable
                 DurableWorkPolicy. RepoNode. WorktreeNode. TaskNode.
                 DurableGovernedStrategy.))

(export MarlinRepo.
        MarlinWorktree.
        marlin-derive-worktree
        marlin-derive-task
        marlin-orchestrate-task
        marlin-replan-task-epoch
        marlin-admit-node-effect
        marlin-envelope-meet
        marlin-invalidate-premises
        marlin-workspace-plan
        marlin-apply-org-overlay
        marlin-reanchor-mainline
        marlin-reconcile-detour-exit
        marlin-perceive-claims
        marlin-assert-write-owner
        marlin-claim-node!
        marlin-extend-node!
        marlin-handover-node!
        marlin-record-task-cas!
        marlin-recover-task
        marlin-commit-auto-archive!
        marlin-reopen-task!
        marlin-project-workbench-tabs)

(.def (MarlinRepo. @ RepoNode.)
  repo-id: "marlin-agent-core"
  topology-path: "topology.org"
  index-path: "index.org"
  policy: (.o (:: @ DurableWorkPolicy.)
              focused-tab-archive: 'on-blur
              scratch-promotion: 'on-plan
              reopen-mode: 'peek-until-edit
              max-concurrency: 4)
  strategy: DurableGovernedStrategy.)

(.def (MarlinWorktree. @ [MarlinRepo. WorktreeNode.])
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
    ((.ref LabContextDurable 'orchestration-plan) task org-text jstate)))

(def (marlin-replan-task-epoch request)
  (let* ((task (.ref request 'task))
         (prior-source (.ref request 'prior-source))
         (next-source (.ref request 'next-source))
         (completed-cas (opt-ref request 'completed-cas #f))
         (in-flight (opt-ref request 'in-flight-ids '()))
         (ctx (opt-ref request 'governance-context (.o)))
         (dag-fn (.ref LabContextDurable 'funflow-dag))
         (replan-fn (.ref (.ref LabFunflow 'durable-dag) 'replan-epoch))
         (prior-dag (.ref (dag-fn task prior-source) 'dag))
         (next-dag (.ref (dag-fn task next-source) 'dag)))
    (replan-fn prior-dag next-dag
               completed-cas-table: completed-cas
               in-flight-ids: in-flight
               strategy: (opt-ref task 'strategy DurableGovernedStrategy.)
               context: ctx)))

(def (marlin-record-task-cas! request)
  ((.ref LabDurableWork 'record-cas!)
   (.ref request 'journal)
   (.ref request 'task-id)
   (.ref request 'node-id)
   (.ref request 'fingerprint)
   (.ref request 'result-digest)))

(def (marlin-recover-task request)
  ((.ref LabContextDurable 'recover)
   (.ref request 'journal)
   (.ref request 'task)
   (.ref request 'source)
   in-flight-ids: (opt-ref request 'in-flight-ids '())
   unclosed-attempts: (opt-ref request 'unclosed-attempts '())
   governance-context: (opt-ref request 'governance-context (.o))))

(def (marlin-commit-auto-archive! request)
  ((.ref LabContextDurable 'commit-archive!)
   (.ref request 'journal)
   (.ref request 'task)
   (.ref request 'source)
   (opt-ref request 'index-source "")
   (opt-ref request 'journal-state (.o))
   (opt-ref request 'timestamp "2026-11-24 Tue 00:00")))

(def (marlin-reopen-task! request)
  ((.ref LabContextDurable 'reopen!)
   (.ref request 'journal)
   (.ref request 'task)
   (.ref request 'source)
   (opt-ref request 'index-source "")
   (opt-ref request 'timestamp "2026-11-24 Tue 00:00")))

(def (marlin-project-workbench-tabs request)
  ((.ref LabDurableWork 'project-workbench-tabs)
   (opt-ref request 'owner MarlinWorktree.)
   (.ref request 'tasks)
   (opt-ref request 'journal-state (.o))
   (opt-ref request 'peek-set '())
   (opt-ref request 'focused-task-id #f)))

(def (marlin-admit-node-effect request)
  ((.ref (.ref LabFunflow 'durable-dag) 'admit-node-effect)
   (.ref request 'node)
   effect-class: (opt-ref request 'effect-class #f)
   envelope: (opt-ref request 'envelope (.o))))

(def (marlin-envelope-meet request)
  ((.ref (.ref LabFunflow 'durable-dag) 'envelope-meet)
   (.ref request 'parent)
   (.ref request 'overlay)))

(def (marlin-invalidate-premises request)
  ((.ref (.ref LabFunflow 'durable-dag) 'invalidate-premises)
   (.ref request 'dag)
   (opt-ref request 'live-premises '())))

(def (marlin-workspace-plan request)
  ((.ref (.ref LabFunflow 'durable-dag) 'workspace-plan)
   (.ref request 'topology)
   remote-heads: (opt-ref request 'remote-heads '())))

(def (marlin-apply-org-overlay request)
  ((.ref LabDurableWork 'apply-overlay)
   (.ref request 'task)
   (.ref request 'overlay)))

(def (marlin-reanchor-mainline request)
  ((.ref LabContextDurable 'reanchor-mainline)
   (.ref request 'task)
   (.ref request 'source)
   mainline-core: (opt-ref request 'mainline-core '())
   max-detour-depth: (opt-ref request 'max-detour-depth 2)))

(def (marlin-reconcile-detour-exit request)
  ((.ref LabDurableWork 'reconcile-detour-exit)
   (.ref request 'detour)
   (.ref request 'exit-state)
   (.ref request 'anchor)))

(def (marlin-perceive-claims request)
  ((.ref LabContextDurable 'perceive-claims)
   (.ref request 'source)))

(def (marlin-assert-write-owner request)
  ((.ref LabContextDurable 'assert-write-owner)
   (.ref request 'source)
   (.ref request 'node-id)
   (.ref request 'actor)))

(def (marlin-claim-node! request)
  ((.ref LabContextDurable 'claim!)
   (opt-ref request 'journal #f)
   (.ref request 'task-id)
   (.ref request 'node-id)
   (.ref request 'actor)
   worktree: (opt-ref request 'worktree "main")
   scope-digest: (opt-ref request 'scope-digest "")
   now: (opt-ref request 'now 0)
   ttl: (opt-ref request 'ttl 120)
   org-source: (.ref request 'source)))

(def (marlin-extend-node! request)
  ((.ref LabContextDurable 'extend!)
   (opt-ref request 'journal #f)
   (.ref request 'task-id)
   (.ref request 'parent-node-id)
   (.ref request 'new-node-id)
   (.ref request 'actor)
   (.ref request 'title)
   body: (opt-ref request 'body "")
   role: (opt-ref request 'role "extension")
   now: (opt-ref request 'now 0)
   org-source: (.ref request 'source)))

(def (marlin-handover-node! request)
  ((.ref LabContextDurable 'handover!)
   (opt-ref request 'journal #f)
   (.ref request 'task-id)
   (.ref request 'node-id)
   (.ref request 'next-actor)
   now: (opt-ref request 'now 0)
   force-explicit?: (opt-ref request 'force-explicit? #f)
   custody-oid: (opt-ref request 'custody-oid #f)
   org-source: (.ref request 'source)))
