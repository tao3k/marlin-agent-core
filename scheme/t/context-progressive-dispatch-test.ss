;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .get .o .ref)
        (only-in :std/list/list find)
        (only-in :std/string/misc string-contains)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
(export context-progressive-dispatch-test)

(def ContextSys (.get marlin-agent-core.init.exports Marlin context-system))
(def Progressive (.get ContextSys progressive))
(def AdmitOrg (.get ContextSys org-contract-source))

(def project-rules-org
  (string-append
   ":PROPERTIES:\n"
   ":ID: marlin-context-root\n"
   ":CONTRACT_ID: context.gitops.artifacts.index.v1\n"
   ":CONTRACT_REVISION: 1\n"
   ":PROJECT_ID: marlin-agent-core\n"
   ":END:\n"
   "* Progressive Policy\n"
   "#+begin_src scheme :org-progressive-rules\n"
   "(split-line-watermark 750)\n"
   "(default-focus-depth 2)\n"
   "(require-sod-reviewer #t)\n"
   "(fold-thinking-by-default #t)\n"
   "(allow-agent-drill-down #t)\n"
   "#+end_src\n"))

(def arch-org
  (string-append
   ":PROPERTIES:\n"
   ":ID: arch-root-01\n"
   ":CONTRACT_ID: context.governance.work.v1\n"
   ":CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":END:\n"
   "* Architecture: Progressive Context :ATTACH:\n"
   ":PROPERTIES:\n"
   ":ID: sec-arch-01\n"
   ":AGENT_ID: agent/architect-01\n"
   ":AGENT_TIER: high\n"
   ":REVIEWER_AGENT_ID: agent/verifier-01\n"
   ":TASK_ID: task/arch-01\n"
   ":END:\n"
   "#+NAME: arch-1\n"
   "#+BEGIN_SRC llm :model claude-opus-4-6 :thinking high :results drawer replace\n"
   "Define the progressive Org -> GQL + ASCENT architecture.\n"
   "#+END_SRC\n\n"
   "#+RESULTS: arch-1\n"
   ":RESULTS:\n"
   "#+BEGIN_THINKING\n"
   "Deep architectural tradeoff reasoning folded at L2.\n"
   "#+END_THINKING\n"
   "Diagram: [[attachment:progressive-flow.png]].\n"
   ":END:\n"))

(def session-org
  (string-append
   ":PROPERTIES:\n"
   ":ID: tmp-0042-root\n"
   ":CONTRACT_ID: context.session.seed.v1\n"
   ":CONTRACT_REVISION: 1\n"
   ":GITOPS_PROJECT: marlin-agent-core\n"
   ":END:\n"
   "* TODO [#A] Implement Marlin Progressive Dispatch [0/2]\n"
   "SCHEDULED: <2026-11-24 Tue 10:00>\n"
   ":PROPERTIES:\n"
   ":ID: task-marlin-01\n"
   ":AGENT_ID: agent/worker-01\n"
   ":AGENT_TIER: mid\n"
   ":REVIEWER_AGENT_ID: agent/architect-01\n"
   ":TASK_ID: task/marlin-01\n"
   ":DEPENDS_ON: sec-arch-01\n"
   ":END:\n"
   "- [ ] Expose progressive facade through scheme/context-system\n"
   "- [ ] Verify org-attach resolution and two-stage repair\n\n"
   "#+NAME: turn-01\n"
   "#+BEGIN_SRC llm :model qwen-coder-plus :thinking low :results drawer replace\n"
   "Wire LabContext progressive dispatch into Marlin.\n"
   "#+END_SRC\n"))

(def context-progressive-dispatch-test
  (test-suite "Marlin Production Context Progressive Dispatch, GQL & Two-Stage Repair"

    (test-case "Progressive dispatch combines project rules, GQL ready tasks, ASCENT L0..L3, drill-down, facets, and org-attach paths"
      (let* ((arch-cut (AdmitOrg (.o identity: "shared/arch.org" generation: 1 source: arch-org)))
             (sess-cut (AdmitOrg (.o identity: "tmp/0042.org" generation: 1 source: session-org)))
             (dispatch ((.get Progressive dispatch)
                        (.o sources: (list arch-cut sess-cut)
                            context-org: project-rules-org
                            owner-root: "/tmp/marlin-owner-repo"
                            focus-property-ids: '("task-marlin-01")
                            case-signals: '(cross-module)
                            tier-filter: "mid")))
             (drilled ((.get Progressive dispatch)
                       (.o sources: (list arch-cut sess-cut)
                           context-org: project-rules-org
                           owner-root: "/tmp/marlin-owner-repo"
                           focus-property-ids: '("task-marlin-01")
                           drill-downs: '(("sec-arch-01" . 3))
                           case-signals: '(cross-module)
                           tier-filter: "mid")))
             (ready-slices (.ref (.ref dispatch 'ready-tasks) 'slices))
             (attachments (.ref dispatch 'resolved-attachments))
             (facets (map (cut .ref <> 'facetId)
                          (.ref (.ref dispatch 'progressive-plan) 'triggeredFacets))))
        (check (.ref dispatch 'kind) => 'marlin.context.progressive-dispatch.v1)
        (check (.ref (.ref dispatch 'rule-config) 'split-line-watermark) => 750)
        (check (length ready-slices) => 1)
        (check (.ref (car ready-slices) 'propertyId) => "task-marlin-01")
        ;; L2 hides `#+BEGIN_THINKING` on ADR; L3 drill-down reveals it
        (check (not (string-contains (.ref dispatch 'progressive-org)
                                     "Deep architectural tradeoff reasoning folded at L2."))
               => #t)
        (check (and (string-contains (.ref drilled 'progressive-org)
                                     "Deep architectural tradeoff reasoning folded at L2.") #t)
               => #t)
        ;; `org-attach` link resolves against GitOps owner root (`data/<id>/<file>`)
        (check (length attachments) => 1)
        (check (.ref (car attachments) 'resolved-path)
               => "/tmp/marlin-owner-repo/data/sec-arch-01/progressive-flow.png")
        ;; Dynamic Flash facets inferred by ASCENT
        (check (and (memq 'invariant-section facets) #t) => #t)
        (check (and (memq 'sod-reviewer facets) #t) => #t)
        (check (and (memq 'attach-media facets) #t) => #t)))

    (test-case "Two-stage Org repair fixes mechanical defects at 0 tokens and parser-verified updaters advance turn & fission"
      (let* ((broken
              (string-append
               "*TODO Quick fix [0/1] :ATTACH\n"
               ":PROPERTIES:\n"
               ":ID: qfix-1\n"
               ":END:\n"
               "-[x] Done step\n"
               "#+NAME: t1\n"
               "#+BEGIN_SRC llm :model flash :results drawer replace\n"
               "Check.\n"
               "#+END_SRC\n"
               "#+RESULTS: t1\n"
               ":RESULTS:\n"
               ":THINKING:\n"
               "Auto-fixed nested thinking.\n"
               ":END:\n"
               "OK.\n"))
             (repaired ((.get Progressive repair-org)
                        (.o source-id: "tmp/0099.org"
                            generation: 1
                            source: broken
                            defaults: (.o root-id: "root-99"
                                          contract-id: "context.session.seed.v1"
                                          contract-revision: "1"
                                          project-id: "marlin-agent-core")))))
        (check (.ref repaired 'accepted?) => #t)
        (check (.ref repaired 'stage) => 'ast-auto-repaired)
        (check (and (string-contains (.ref repaired 'repairedOrg) "* TODO Quick fix [1/1] :ATTACH:") #t) => #t)
        ;; Advance `session-org` turn -> checklist -> DONE -> safe zero-compaction fission
        (let* ((u1 ((.get Progressive babel-upsert)
                    (.o source: session-org
                        block-name: "turn-01"
                        body: "Progressive dispatch wired."
                        thinking: "Verified LabContext exports.")))
               (u2 ((.get Progressive checklist-set)
                    (.o source: u1 headline-property-id: "task-marlin-01" item-index: 0 checked?: #t)))
               (u3 ((.get Progressive checklist-set)
                    (.o source: u2 headline-property-id: "task-marlin-01" item-index: 1 checked?: #t)))
               (u4 ((.get Progressive headline-set-todo)
                    (.o source: u3 headline-property-id: "task-marlin-01" keyword: "DONE")))
               (cut-done (AdmitOrg (.o identity: "tmp/0042.org" generation: 2 source: u4)))
               (fission ((.get Progressive fission-subtree)
                         (.o scratch-cut: cut-done
                             target-org: arch-org
                             target-property-id: "task-marlin-01"))))
          (check (and (string-contains u4 "* DONE [#A] Implement Marlin Progressive Dispatch [2/2]") #t) => #t)
          (check (and (string-contains (.ref fission 'promotedSubtree) ":CONTEXT_ORIGIN_SOURCE: tmp/0042.org") #t) => #t)
          (check (and (string-contains (.ref fission 'targetOrg) ":ID: task-marlin-01") #t) => #t))))))
