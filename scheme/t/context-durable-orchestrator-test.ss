;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Marlin production tests for the declarative Durable Task Orchestrator,
;;; C4 POO Repo/Worktree/Task derivation, Tweag-inspired FunFlow CAS DAG
;;; replanning, atomic Auto-Archive, and pure Task-as-Tab WorkbenchTabProjection.

(import :std/test
        (only-in :std/io/tempfile make-temporary-file-name)
        (only-in :std/string/misc string-contains)
        (only-in :clan/poo/object .get .o .ref)
        (only-in :marlin-agent-core/interface LabDurableJournal DurableTaskPolicy.)
        (only-in :marlin-agent-core/context-system/config MarlinContextSystemScope))

(export context-durable-orchestrator-test)

(def orch (.get MarlinContextSystemScope orchestrator))

(def marlin-task-active-org
  (string-append
   ":PROPERTIES:\n"
   ":ORG_CONTRACT: context.session.seed.v1\n"
   ":ORG_CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":GITOPS_WORKTREE: wt/task-tabs\n"
   ":END:\n"
   "* IN_PROGRESS Build Declarative Orchestrator [1/2]\n"
   ":PROPERTIES:\n"
   ":ID: task-orch-01\n"
   ":AGENT_ID: agent/architect\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n"
   "- [X] Lock RFC 40.183 & SysML\n"
   "- [ ] Wire Marlin Scheme bindings\n\n"
   "** DONE Formalize SysML\n"
   ":PROPERTIES:\n"
   ":ID: step-sysml\n"
   ":AGENT_ID: agent/architect\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n\n"
   "** TODO Wire Marlin interface\n"
   ":PROPERTIES:\n"
   ":ID: step-scheme\n"
   ":DEPENDS_ON: step-sysml\n"
   ":AGENT_ID: agent/builder\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n"))

(def marlin-task-replanned-org
  (string-append
   ":PROPERTIES:\n"
   ":ORG_CONTRACT: context.session.seed.v1\n"
   ":ORG_CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":GITOPS_WORKTREE: wt/task-tabs\n"
   ":END:\n"
   "* IN_PROGRESS Build Declarative Orchestrator [1/3]\n"
   ":PROPERTIES:\n"
   ":ID: task-orch-01\n"
   ":AGENT_ID: agent/architect\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n"
   "- [X] Lock RFC 40.183 & SysML\n"
   "- [ ] Migrate FunFlow into Lab\n"
   "- [ ] Wire Marlin Scheme bindings\n\n"
   "** DONE Formalize SysML\n"
   ":PROPERTIES:\n"
   ":ID: step-sysml\n"
   ":AGENT_ID: agent/architect\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n\n"
   "** TODO Migrate FunFlow into Lab\n"
   ":PROPERTIES:\n"
   ":ID: step-funflow\n"
   ":DEPENDS_ON: step-sysml\n"
   ":AGENT_ID: agent/builder\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n\n"
   "** TODO Wire Marlin interface\n"
   ":PROPERTIES:\n"
   ":ID: step-scheme\n"
   ":DEPENDS_ON: step-sysml step-funflow\n"
   ":AGENT_ID: agent/builder\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n"))

(def marlin-task-done-org
  (string-append
   ":PROPERTIES:\n"
   ":ORG_CONTRACT: context.session.seed.v1\n"
   ":ORG_CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":GITOPS_WORKTREE: wt/task-tabs\n"
   ":END:\n"
   "* DONE Build Declarative Orchestrator [2/2]\n"
   ":PROPERTIES:\n"
   ":ID: task-orch-01\n"
   ":AGENT_ID: agent/architect\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n"
   "- [X] Lock RFC 40.183 & SysML\n"
   "- [X] Wire Marlin Scheme bindings\n\n"
   "** DONE Formalize SysML\n"
   ":PROPERTIES:\n"
   ":ID: step-sysml\n"
   ":AGENT_ID: agent/architect\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n\n"
   "** DONE Wire Marlin interface\n"
   ":PROPERTIES:\n"
   ":ID: step-scheme\n"
   ":DEPENDS_ON: step-sysml\n"
   ":AGENT_ID: agent/builder\n"
   ":REVIEWER_AGENT_ID: agent/reviewer\n"
   ":END:\n"))

(def context-durable-orchestrator-test
  (test-suite "Marlin Declarative Durable Orchestrator and Task-as-Tab Projection"

    (test-case "C4 worktree/task derivation, CAS epoch replanning, atomic Auto-Archive, and Tab projection"
      (let* ((wt ((.ref orch 'derive-worktree)
                  (.o bind-these: #f
                      worktree: "wt/task-tabs"
                      branch: "feat/task-tabs"
                      source-cut: "sha256:marlin-cut-1"
                      policy: (.o (:: @ DurableTaskPolicy.)
                                  require-sod-reviewer: #t
                                  focused-tab-archive: 'on-blur))))
             (task ((.ref orch 'derive-task)
                    (.o bind-these: #f
                        worktree-node: wt
                        task-id: "task-orch-01"
                        org-path: "work/task-orch-01.org")))
             (plan ((.ref orch 'orchestrate-task)
                    (.o bind-these: #f task: task source: marlin-task-active-org)))
             (dag (.ref (.ref plan 'dag-projection) 'dag))
             (fp-sysml (cdr (assq 'step-sysml (.ref dag 'fingerprints))))
             (jpath (make-temporary-file-name "marlin-durable-orch-"))
             (journal ((.ref LabDurableJournal 'open) jpath)))
        (dynamic-wind void
          (lambda ()
            (check-equal? (.ref plan 'ready-to-schedule) '(step-scheme))
            (check-equal? (.ref plan 'ready-to-auto-archive?) #f)
            ;; Record CAS for step-sysml and verify epoch replanning preserves CAS while inserting step-funflow
            ((.ref orch 'record-cas!)
             (.o bind-these: #f
                 journal: journal
                 task-id: "task-orch-01"
                 node-id: 'step-sysml
                 fingerprint: fp-sysml
                 result-digest: "sha256:sysml-ok"))
            (let* ((replan ((.ref orch 'replan-epoch)
                            (.o bind-these: #f
                                task: task
                                prior-source: marlin-task-active-org
                                next-source: marlin-task-replanned-org
                                completed-cas: (list (cons 'step-sysml fp-sysml)))))
                   (rplan (.ref replan 'resume-plan)))
              (check-equal? (.ref replan 'valid?) #t)
              (check-equal? (.ref replan 'preserved-cas-nodes) '(step-sysml))
              (check-equal? (.ref replan 'added-nodes) '(step-funflow))
              (check-equal? (.ref rplan 'ready-frontier) '(step-funflow))
              (check-equal? (.ref rplan 'blocked-nodes) '(step-scheme)))
            ;; Recovery snapshot finds persisted CAS entry
            (let ((recov ((.ref orch 'recover-task)
                          (.o bind-these: #f journal: journal task: task source: marlin-task-active-org))))
              (check-equal? (.ref recov 'completed-cas-count) 1))
            ;; Atomic Auto-Archive updates both Task Org and Plan-A index.org
            (let ((archived ((.ref orch 'commit-auto-archive!)
                             (.o bind-these: #f
                                 journal: journal
                                 task: task
                                 source: marlin-task-done-org
                                 index-source: ""
                                 timestamp: "2026-11-24 Tue 00:30"))))
              (check-equal? (.ref archived 'committed?) #t)
              (check-equal? (and (string-contains (.ref archived 'next-task-org) ":ARCHIVE:") #t) #t)
              (check-equal? (and (string-contains (.ref archived 'next-index-org) ":ARCHIVE:") #t) #t)
              ;; Pure Task-as-Tab projection
              (let* ((tabs ((.ref orch 'project-workbench-tabs)
                            (.o bind-these: #f
                                owner: wt
                                tasks: (list (.o bind-these: #f
                                                 task-id: "task-orch-01"
                                                 title: "Build Declarative Orchestrator"
                                                 status: 'done
                                                 archived?: #t
                                                 checklist-done: 2
                                                 checklist-total: 2)
                                             (.o bind-these: #f
                                                 task-id: "task-tui-02"
                                                 title: "TUI Org Window"
                                                 status: 'todo
                                                 archived?: #f
                                                 checklist-done: 0
                                                 checklist-total: 3))
                                peek-set: '()
                                focused-task-id: "task-tui-02")))
                     (reopened ((.ref orch 'reopen-task!)
                                (.o bind-these: #f
                                    journal: journal
                                    task: task
                                    source: (.ref archived 'next-task-org)
                                    index-source: (.ref archived 'next-index-org)
                                    timestamp: "2026-11-24 Tue 00:35"))))
                (check-equal? (.ref tabs 'visible-tab-ids) '(index "task-tui-02"))
                (check-equal? (length (.ref tabs 'archived-index)) 1)
                (check-equal? (.ref reopened 'reopened?) #t)
                (check-equal? (not (string-contains (.ref reopened 'next-task-org) ":ARCHIVE:")) #t)
                (check-equal? (and (string-contains (.ref reopened 'next-task-org) "* TODO") #t) #t))))
          (lambda ()
            ((.ref LabDurableJournal 'close!) journal)
            (when (file-exists? jpath) (delete-file jpath))))))))
