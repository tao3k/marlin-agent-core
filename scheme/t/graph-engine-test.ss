;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Marlin GraphEngine end-to-end integration test:
;;;   1. Mounts `(graph-engine +sqlite-turso)` from `marlin-agent-core/init.ss`
;;;      consuming `agentic-ai-lab` as a pure Library;
;;;   2. Exercises Session-free `TaskActor`, `open-focus` -> `promote`, `detached`
;;;      scope isolation, and C4 mid-flight Steering (`D -> {D1, D2} -> F`);
;;;   3. Projects directly into Marlin's checked Task Workbench `UiPackage`
;;;      (pinned `Index` + active task tabs + read-only ID peek tab);
;;;   4. Projects admitted & blocked `MarlinDispatchEnvelope`s onto Marlin's
;;;      Rust-facing `marlin-graph-loop-continuation-next-action` native ABI;
;;;   5. Executes live `+sqlite-turso` WAL dispatch round-trip and verifies
;;;      `fencing-epoch` anti-split-brain rejection on stale Marlin receipts.

(import :std/test
        (only-in :std/list/list find)
        (only-in :clan/poo/object .get .ref .slot?)
        (only-in :tao3k/agentic-ai-lab/modules/graph-engine/obligation-graph
                 make-obligation make-evidence-fact make-obligation-graph)
        (only-in :tao3k/agentic-ai-lab/modules/graph-engine/delegation-fence
                 make-capability-envelope)
        (only-in :marlin-agent-core/init Marlin)
        (only-in :marlin-agent-core/graph-engine/funcs
                 marlin-graph-engine-make-demo-fleet-vector
                 marlin-graph-engine-vector->atomic-receipt)
        (only-in :marlin-agent-core/graph-engine/fleet-controller
                 make-marlin-fleet-workbench-controller
                 marlin-fleet-workbench-controller-apply-action
                 marlin-fleet-workbench-controller-audit-capsule)
        (only-in :marlin-agent-core/ui/task-workbench
                 make-marlin-task-workbench-package)
        (only-in :marlin-agent-core/policy/projection/graph/continuation
                 marlin-graph-loop-continuation-type-id
                 marlin-graph-loop-continuation-schema-id))

(export marlin-graph-engine-test)

(def ge (.get Marlin graph-engine))

(def (make-sample-trunk trunk-id apex-id)
  (let* ((env (make-capability-envelope
               "marlin-owner"
               allowed-caps: '(context-read repo-scan verify-build)
               denied-caps: '(host-egress)
               egress-allowed?: #f))
         (ob-a (make-obligation "ob-a"
                                title: "Gather Context"
                                required-evidence-ids: '("ev-a")))
         (ob-apex (make-obligation apex-id
                                   title: "Complete Mission"
                                   required-evidence-ids: '("ev-a")
                                   depends-on: '("ob-a")))
         (g (make-obligation-graph
             (string-append "g-" trunk-id)
             cut: 1
             obligations: (list ob-a ob-apex)
             evidence: '()
             det-tasks: '())))
    ((.ref ge 'make-context-trunk)
     trunk-id apex-id g env
     shared-context-cut: 1
     kv-prefix-digest: "sha256:marlin-structural-kv")))

(def marlin-graph-engine-test
  (test-suite "Marlin GraphEngine Integration (UI Workbench + Rust Continuation ABI + Turso WAL)"

    (test-case "Marlin mounts GraphEngine + sqlite-turso and projects Session-free TaskActors into UiPackage"
      (let* ((t-ui (make-sample-trunk "trunk-ui" "ob-apex-ui"))
             (t-done (make-sample-trunk "trunk-done" "ob-apex-done"))
             (a-ui ((.ref ge 'make-task-actor) "task-ui" t-ui '(scope/marlin) title: "Marlin UI Task"))
             (a-done ((.ref ge 'make-task-actor) "task-done" t-done '(scope/marlin) title: "Failed Task"))
             (idx0 ((.ref ge 'make-task-actor-index)))
             (settled ((.ref ge 'settle-task-actor) a-done idx0 outcome: 'failed reason-note: "check failed"))
             (idx1 (.ref settled 'index))
             (ui-res ((.ref ge 'project-workbench-ui)
                      (list a-ui (.ref settled 'actor))
                      idx1
                      peek-set: '("task-done")
                      disclosure-level: 'L2
                      package-builder: make-marlin-task-workbench-package)))
        (check (.slot? ge 'sqlite-turso) => #t)
        (check (.ref ui-res 'session-free?) => #t)
        (check (.ref (.ref ui-res 'tab-projection) 'visible-tab-ids)
               => '(index "task-ui" "task-done"))
        (check (object? (.ref ui-res 'ui-package)) => #t)))

    (test-case "C4 Steering (D -> {D1, D2} -> F) projects blocked escalation and admitted continue_with_graph ABI"
      (let* ((trunk (make-sample-trunk "trunk-dfg" "ob-apex"))
             (a0 ((.ref ge 'make-task-actor) "task-dfg" trunk '(scope/marlin)))
             (m1 ((.ref ge 'make-steering-message)
                  "steer-1" 'branch "Checklist D1"
                  c4-priority: 1 anchor-node-id: "D" join-node-id: "F"
                  branch-id: "D1" target-obligation-id: "ob-a"))
             (a1 (.ref ((.ref ge 'steer-task-actor) a0 m1) 'actor))
             ;; Blocked continue-trunk -> Rust native ABI `escalate_to_human`!
             (d-blocked ((.ref ge 'make-marlin-dispatch) a1 operation: 'continue-trunk join-node-id: "F"))
             (cont-blocked ((.ref ge 'dispatch->continuation) d-blocked))
             ;; Admitted execute-branch -> Rust native ABI `continue_with_graph`!
             (d-branch ((.ref ge 'make-marlin-dispatch) a1 operation: 'execute-branch substrate-target: 'sqlite-turso))
             (cont-branch ((.ref ge 'dispatch->continuation) d-branch)))
        (check (.get cont-blocked type_id) => marlin-graph-loop-continuation-type-id)
        (check (.get cont-blocked schema_id) => marlin-graph-loop-continuation-schema-id)
        (check (.get (.get cont-blocked action) kind) => "escalate_to_human")
        (check (.get (.get cont-blocked action) reason) => "required-branches-still-in-flight")
        (check (.get (.get cont-branch action) kind) => "continue_with_graph")
        (check (.get (.get (.get cont-branch action) compiled_graph) graph_id)
               => "graph-engine:task-dfg")))

    (test-case "Live +sqlite-turso execution round-trip reconciles fresh receipt and rejects stale fencing-epoch"
      (let* ((db-path (string-append "/tmp/marlin-graph-engine-test-"
                                     (number->string (inexact->exact (floor (time->seconds (current-time)))))
                                     ".db"))
             (turso (.ref ge 'sqlite-turso))
             (trunk (make-sample-trunk "trunk-live" "ob-apex"))
             (a0 ((.ref ge 'make-task-actor) "task-live" trunk '(scope/marlin)))
             (m1 ((.ref ge 'make-steering-message)
                  "steer-1" 'branch "Checklist D1"
                  c4-priority: 1 anchor-node-id: "D" join-node-id: "F"
                  branch-id: "D1" target-obligation-id: "ob-a"))
             (a1 (.ref ((.ref ge 'steer-task-actor) a0 m1) 'actor))
             (jh ((.ref ge 'plan-trunk-journal-handoff)
                  "ev:marlin-live-1" "branch-D1" (.ref a1 'trunk)))
             (d-ok ((.ref ge 'make-marlin-dispatch)
                    a1
                    operation: 'execute-branch
                    journal-handoff: jh
                    substrate-target: 'sqlite-turso))
             (env (.ref d-ok 'envelope))
             (j ((.ref turso 'open) db-path)))
        (try
          (let* ((res ((.ref ge 'execute-turso-dispatch!)
                       j a1 env
                       join-node-id: "F"
                       branch-outcomes: '(("D1" . completed))
                       executed-wave-ids: '("wave-0")))
                 (a2 (.ref res 'actor))
                 (d-cont ((.ref ge 'make-marlin-dispatch) a2 operation: 'continue-trunk join-node-id: "F"))
                 (cont-next ((.ref ge 'dispatch->continuation) d-cont))
                 ;; Mid-flight C4 Trunk Steering bumps fencing-epoch -> old receipt MUST be rejected!
                 (pivot ((.ref ge 'make-steering-message) "steer-pivot" 'trunk "Pivot trunk" c4-priority: 0))
                 (a-pivoted (.ref ((.ref ge 'steer-task-actor) a1 pivot) 'actor))
                 (stale-check ((.ref ge 'reconcile-marlin-receipt)
                               a-pivoted (.ref res 'receipt) join-node-id: "F")))
            (check (.ref res 'accepted?) => #t)
            (check (> (.ref (.ref res 'receipt) 'journal-epoch) 0) => #t)
            (check (.get (.get cont-next action) kind) => "continue_with_graph")
            (check (.ref stale-check 'accepted?) => #f)
            (check (.ref stale-check 'reason) => 'stale-fencing-epoch)
            (displayln "MARLIN-GRAPH-ENGINE-INTEGRATION-OK"))
          (finally
            ((.ref turso 'close!) j)
            (when (file-exists? db-path) (delete-file db-path))
            (when (file-exists? (string-append db-path "-wal"))
              (delete-file (string-append db-path "-wal")))
            (when (file-exists? (string-append db-path "-shm"))
              (delete-file (string-append db-path "-shm"))))))

    (test-case "task-sandbox-verify emits sandbox-run directly and escalates oscillation to human-inbox"
      (let* ((fleet0 (marlin-graph-engine-make-demo-fleet-vector '#("p1" "s1" "sha256:kv")))
             (ctrl0 (make-marlin-fleet-workbench-controller fleet0 disclosure-level: 'L1))
             (cap0 (marlin-fleet-workbench-controller-audit-capsule ctrl0))
             (ctrl1 (marlin-fleet-workbench-controller-apply-action
                     ctrl0 'task-sandbox-verify arg: "sib-deny:/bin/sh:forbidden"))
             (effs1 (.get ctrl1 last-effects))
             (sbx1 (find (lambda (e) (eq? (.ref e 'kind) 'sandbox-run)) effs1))
             (llm1 (find (lambda (e) (eq? (.ref e 'kind) 'llm-complete)) effs1))
             (rcpt1 (marlin-graph-engine-vector->atomic-receipt
                     (vector 'sandbox-exited (.ref sbx1 'effect-id) 126 "" "sandbox-denied: /bin/sh")))
             (ctrl2 (marlin-fleet-workbench-controller-apply-action
                     ctrl1 'task-fleet-step receipts: (list rcpt1)))
             (sbx2 (find (lambda (e) (eq? (.ref e 'kind) 'sandbox-run)) (.get ctrl2 last-effects)))
             (rcpt2 (marlin-graph-engine-vector->atomic-receipt
                     (vector 'sandbox-exited (.ref sbx2 'effect-id) 126 "" "sandbox-denied: /bin/sh")))
             (ctrl3 (marlin-fleet-workbench-controller-apply-action
                     ctrl2 'task-fleet-step receipts: (list rcpt2)))
             (sbx3 (find (lambda (e) (eq? (.ref e 'kind) 'sandbox-run)) (.get ctrl3 last-effects)))
             (cap3 (marlin-fleet-workbench-controller-audit-capsule ctrl3)))
        (check (object? sbx1) => #t)
        (check (.ref sbx1 'program) => "/bin/sh")
        (check (.ref sbx1 'args) => '("forbidden"))
        (check llm1 => #f)
        (check (object? sbx2) => #t)
        (check sbx3 => #f)
        (check (.ref cap0 'total-human-inbox-items) => 1)
        (check (.ref cap3 'total-human-inbox-items) => 2)
        (check (.ref cap3 'authenticates-evidence?) => #f)
        (check (.ref cap3 'authorizes-effect?) => #f)))

    (test-case "cross-focus task-human-approve resolves parent defeater after task-reopen shifts focus"
      (let* ((fleet0 (marlin-graph-engine-make-demo-fleet-vector '#("p1" "s1" "sha256:kv")))
             (ctrl0 (make-marlin-fleet-workbench-controller fleet0 disclosure-level: 'L1))
             (ctrl-sbx (marlin-fleet-workbench-controller-apply-action
                        ctrl0 'task-sandbox-verify arg: "sib-ok:/bin/echo:verified"))
             (sbx-ok (find (lambda (e) (eq? (.ref e 'kind) 'sandbox-run)) (.get ctrl-sbx last-effects)))
             (rcpt-ok (marlin-graph-engine-vector->atomic-receipt
                       (vector 'sandbox-exited (.ref sbx-ok 'effect-id) 0 "verified\n" "")))
             (ctrl-settled (marlin-fleet-workbench-controller-apply-action
                            ctrl-sbx 'task-fleet-step receipts: (list rcpt-ok) auto-archive?: #t))
             (cap-settled (marlin-fleet-workbench-controller-audit-capsule ctrl-settled))
             (ctrl-reopened (marlin-fleet-workbench-controller-apply-action
                             ctrl-settled 'task-reopen arg: "sib-ok"))
             (cap-reopened (marlin-fleet-workbench-controller-audit-capsule ctrl-reopened))
             (ctrl-approved (marlin-fleet-workbench-controller-apply-action
                             ctrl-reopened 'task-human-approve arg: "def-turso-01"))
             (cap-approved (marlin-fleet-workbench-controller-audit-capsule ctrl-approved)))
        (check (object? sbx-ok) => #t)
        (check (.ref cap-settled 'archived-actor-count) => 1)
        (check (.ref (.get ctrl-reopened fleet) 'focused-task-id) => "sib-ok")
        (check (.ref cap-reopened 'total-active-defeaters) => 1)
        (check (.ref cap-approved 'total-active-defeaters) => 0)
        (check (.ref cap-approved 'favored-path-collapsed?) => #f)
        (check (.ref cap-approved 'authorizes-effect?) => #f))))))
