;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme TUI regression tests for Layer 2 Task-as-Tab Workbench Projection
;;; (RFC 40.183 §7.6): Pinned Index + Dynamic Active Task Tabs + On-Blur Auto-Hide
;;; + Read-Only Peek + Durable Reopen + Progressive L0..L3 Disclosure keymaps/commands.

(import :std/test
        (only-in :std/list/list find)
        (only-in :clan/poo/object .get .o .ref .slot?)
        (only-in :clan/poo/type TypeError?)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/ui/task-workbench
                 make-marlin-task-workbench-package
                 marlin-task-workbench-peek-set-add
                 marlin-task-workbench-peek-set-remove
                 marlin-task-workbench-next-disclosure
                 marlin-task-workbench-apply-detour-action)
        (only-in :marlin-agent-core/ui/session
                 marlin-ui-session-open
                 marlin-ui-session-project
                 marlin-ui-session-dispatch
                 make-marlin-task-workbench-controller
                 marlin-task-workbench-controller-apply-action
                 marlin-task-workbench-controller-project)
        (only-in :tao3k/agentic-ai-lab/modules/graph-engine/interface
                 make-capability-envelope
                 make-obligation
                 make-deterministic-task-node
                 make-obligation-graph
                 make-context-trunk
                 graph-make-task-actor
                 make-competing-hypothesis
                 make-hypothesis-branch-proposal
                 make-cas-get-effect
                 make-cas-hit-receipt
                 make-cas-miss-receipt
                 make-cas-stored-receipt
                 make-llm-completed-receipt
                 make-effect-error-receipt
                 make-unified-driver-state
                 run-unified-driver-to-quiescence
                 project-driver-state->workbench-projection
                 graph-enter-task-actor-detour
                 make-graph-fleet-state
                 fleet-spawn-sibling-from-detour
                 fleet-apply-bitemporal-event
                 plan-fleet-cas-checkpoint
                 restore-fleet-from-cas-receipt
                 run-graph-fleet-to-quiescence
                 project-fleet->workbench-projection))

(export task-workbench-tabs-test)

(def ctx-sys (.get marlin-agent-core.init.exports Marlin context-system))
(def orch (.ref ctx-sys 'durable))
(def ui-tabs (.get marlin-agent-core.init.exports Marlin ui-centaur-tabs))

(def (bar-segments bar)
  (vector-ref (vector-ref bar 1) 2))

(def (tab-commands bar)
  (map (lambda (seg) (vector-ref seg 5)) (vector->list (bar-segments bar))))

(def (sample-tasks)
  (list (.o bind-these: #f
            task-id: "task-arch-01"
            title: "Ship Design 165"
            status: 'done
            archived?: #t
            worktree: "wt/c4"
            checklist-done: 4
            checklist-total: 4)
        (.o bind-these: #f
            task-id: "task-tui-02"
            title: "Task-as-Tab TUI"
            status: 'in-progress
            archived?: #f
            worktree: "wt/tui"
            checklist-done: 2
            checklist-total: 3)))

(def task-workbench-tabs-test
  (test-suite "Marlin Layer 2 Scheme TUI Task-as-Tab Workbench Projection"

    (test-case "Pinned Index + Dynamic Active Task Tabs hide background archived tasks and render Centaur Tabs"
      (let* ((wt ((.ref orch 'derive-worktree) (.o bind-these: #f worktree: "wt/tui")))
             (proj ((.ref orch 'project-workbench-tabs)
                    (.o bind-these: #f
                        owner: wt
                        tasks: (sample-tasks)
                        peek-set: '()
                        focused-task-id: "task-tui-02")))
             (bar ((.get ui-tabs task-tabs-for) proj 'tabs-bar))
             (cmds (tab-commands bar)))
        ;; Left pinned `index` + right dynamic active `task-tui-02`; background archived `task-arch-01` hidden!
        (check-equal? (.ref proj 'visible-tab-ids) '(index "task-tui-02"))
        (check-equal? (length (.ref proj 'archived-index)) 1)
        (check-equal? cmds '(buffer-index buffer-index buffer-task-tui-02 buffer-task-tui-02))))

    (test-case "Focused archived task obeys on-blur policy and disappears on blur; peek mounts read-only"
      (let* ((wt ((.ref orch 'derive-worktree) (.o bind-these: #f worktree: "wt/tui")))
             (focused-arch
              ((.ref orch 'project-workbench-tabs)
               (.o bind-these: #f
                   owner: wt
                   tasks: (sample-tasks)
                   peek-set: '()
                   focused-task-id: "task-arch-01")))
             (blurred
              ((.ref orch 'project-workbench-tabs)
               (.o bind-these: #f
                   owner: wt
                   tasks: (sample-tasks)
                   peek-set: '()
                   focused-task-id: "task-tui-02")))
             (peek-set (marlin-task-workbench-peek-set-add '() "task-arch-01"))
             (peeked
              ((.ref orch 'project-workbench-tabs)
               (.o bind-these: #f
                   owner: wt
                   tasks: (sample-tasks)
                   peek-set: peek-set
                   focused-task-id: "task-tui-02")))
             (unpeeked
              ((.ref orch 'project-workbench-tabs)
               (.o bind-these: #f
                   owner: wt
                   tasks: (sample-tasks)
                   peek-set: (marlin-task-workbench-peek-set-remove peek-set "task-arch-01")
                   focused-task-id: "task-tui-02"))))
        ;; Focused archived task remains visible under default `on-blur` policy
        (check-equal? (.ref focused-arch 'visible-tab-ids) '(index "task-arch-01" "task-tui-02"))
        ;; Blur hides it automatically
        (check-equal? (.ref blurred 'visible-tab-ids) '(index "task-tui-02"))
        ;; Adding to peek-set mounts it in `peek-readonly` mode
        (check-equal? (.ref peeked 'visible-tab-ids) '(index "task-arch-01" "task-tui-02"))
        (let ((peek-tab (find (lambda (t) (equal? (.ref t 'id) "task-arch-01"))
                              (.ref peeked 'dynamic-tabs))))
          (check-equal? (.ref peek-tab 'mode) 'peek-readonly))
        ;; Removing from peek-set hides it again cleanly
        (check-equal? (.ref unpeeked 'visible-tab-ids) '(index "task-tui-02"))))

    (test-case "Task Workbench UiPackage wires Index/Task buffers, SPC o keymaps, L0..L3 commands, and frame switching"
      (let* ((wt ((.ref orch 'derive-worktree) (.o bind-these: #f worktree: "wt/tui")))
             (proj ((.ref orch 'project-workbench-tabs)
                    (.o bind-these: #f
                        owner: wt
                        tasks: (sample-tasks)
                        peek-set: '("task-arch-01")
                        focused-task-id: "task-tui-02")))
             (pkg-ui (make-marlin-task-workbench-package proj disclosure-level: 'L2))
             (wb ((.get pkg-ui workbench)
                  (.o model: "gemini-2.5-pro"
                      session: "task-workbench"
                      contributions: '()
                      extensions: '())))
             (projected ((.get pkg-ui project-workbench) wb))
             (keys (vector->list ((.get pkg-ui keymap-entries))))
             (frame0 ((.get pkg-ui make-frame)))
             (frame1 ((.get pkg-ui select-buffer) frame0 'index))
             (frame2 ((.get pkg-ui select-buffer) frame1 'task-tui-02))
             (proj-frame ((.get pkg-ui project-frame) frame2)))
        ;; Projected workbench is valid Ratatui native vector
        (check-equal? (vector? projected) #t)
        ;; Frame buffer switching moves cleanly across pinned 'index and dynamic 'task-tui-02
        (check-equal? (vector-ref (vector-ref (vector-ref proj-frame 1) 0) 1) 'task-tui-02)
        ;; SPC o keymap entries registered without colliding with UiMeowScope
        (check-equal? (and (member '#(normal #(SPC o i) task-index) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o t) task-switch) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o p) task-peek) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o r) task-reopen) keys) #t) #t)
        (check-equal? (and (member (vector 'normal (vector 'SPC 'o (string->symbol "0")) 'task-drill-l0) keys) #t) #t)
        (check-equal? (and (member (vector 'normal (vector 'SPC 'o (string->symbol "1")) 'task-drill-l1) keys) #t) #t)
        (check-equal? (and (member (vector 'normal (vector 'SPC 'o (string->symbol "2")) 'task-drill-l2) keys) #t) #t)
        (check-equal? (and (member (vector 'normal (vector 'SPC 'o (string->symbol "3")) 'task-drill-l3) keys) #t) #t)
        ;; Progressive disclosure validator accepts L0..L3 and rejects invalid levels
        (check-equal? (marlin-task-workbench-next-disclosure 'L1 'L3) 'L3)
        (check-exception (marlin-task-workbench-next-disclosure 'L1 'L9) TypeError?)
        ;; Live UiNativeSession executes pure-Scheme scheme-action commands end-to-end
        (let* ((sess0 (marlin-ui-session-open
                        (vector pkg-ui #("gemini-2.5-pro" "task-workbench" #() #()) #())))
               (step1 (marlin-ui-session-dispatch
                        (vector sess0 'scheme-action '#(task-tab-index #f))))
               (sess1 (vector-ref step1 0))
               (step2 (marlin-ui-session-dispatch
                        (vector sess1 'scheme-action '#(task-switch #f))))
               (sess2 (vector-ref step2 0))
               (step3 (marlin-ui-session-dispatch
                        (vector sess2 'scheme-action '#(task-drill-l2 #f)))))
          (check-equal? (vector-ref step1 1) '#(select-buffer index))
          (check-equal? (vector-ref (vector-ref step2 1) 0) 'select-buffer)
          (check-equal? (vector-ref (vector-ref step3 1) 0) 'select-buffer)
          (check-equal? (vector? (marlin-ui-session-project sess2)) #t))))

    (test-case "Scheme-only Scoped Detour 4-Exit controls and Competing Hypothesis L3 view require ZERO Rust edits"
      (let* ((hyps (list (.o bind-these: #f
                             hypothesis-id: "H1"
                             claim: "Compromised CI credential"
                             contradictions: '("clock-skew")
                             open-questions: '("token-rotated?"))
                         (.o bind-these: #f
                             hypothesis-id: "H2"
                             claim: "Release automation drift"
                             contradictions: '()
                             open-questions: '("approver-id?"))))
             (proj0 (.o bind-these: #f
                        pinned-tab: 'index
                        dynamic-tabs:
                        (list (.o bind-these: #f
                                  id: "task-sec-01"
                                  label: "Security Investigation"
                                  status: 'in-progress
                                  mode: 'active
                                  detour-mode: 'mainline
                                  detour-node-id: #f
                                  worktree: "wt/sec"
                                  checklist-done: 1
                                  checklist-total: 3
                                  competing-hypotheses: hyps))
                        archived-index: '()
                        visible-tab-ids: '(index "task-sec-01")))
             ;; Enter blocker-active detour in pure Scheme
             (proj-blk (marlin-task-workbench-apply-detour-action
                        proj0 "task-sec-01" 'task-detour-blocker "blk-ci-token"))
             ;; Exit via spawn-sibling-task -> adds sibling tab dynamically!
             (proj-sib (marlin-task-workbench-apply-detour-action
                        proj-blk "task-sec-01" 'task-detour-exit-sibling "task-sec-sib"))
             (pkg-ui (make-marlin-task-workbench-package proj-sib disclosure-level: 'L3))
             (keys (vector->list ((.get pkg-ui keymap-entries))))
             (sess0 (marlin-ui-session-open
                     (vector pkg-ui #("gemini-2.5-pro" "detour-workbench" #() #()) #())))
             (step-blk (marlin-ui-session-dispatch
                        (vector sess0 'scheme-action '#(task-detour-blocker "blk-ci-token"))))
             (step-sib (marlin-ui-session-dispatch
                        (vector (vector-ref step-blk 0) 'scheme-action '#(task-detour-exit-sibling "task-sec-sib")))))
        (check-equal? (.ref (car (.ref proj-blk 'dynamic-tabs)) 'detour-mode) 'blocker-active)
        (check-equal? (.ref proj-sib 'visible-tab-ids) '(index "task-sec-01" "task-sec-sib"))
        (check-equal? (.ref (car (.ref proj-sib 'dynamic-tabs)) 'detour-mode) 'mainline)
        ;; SPC o d * keybindings registered via generic scheme-action
        (check-equal? (and (member '#(normal #(SPC o d b) task-detour-blocker) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o d s) task-detour-sidequest) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o d r) task-detour-exit-return) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o d a) task-detour-exit-absorb) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o d n) task-detour-exit-sibling) keys) #t) #t)
        (check-equal? (and (member '#(normal #(SPC o d x) task-detour-exit-abort) keys) #t) #t)
        ;; Live UiNativeSession admits and dispatches Detour scheme-actions end-to-end
        (check-equal? (vector-ref (vector-ref step-blk 1) 0) 'select-buffer)
        (check-equal? (vector-ref (vector-ref step-sib 1) 0) 'select-buffer)))

    (test-case "Full Scheme-to-Scheme closure: Lab ASCENT + FunFlow CAS + Competing Hypotheses -> Marlin Declarative TUI (0 Rust edits)"
      (let* ((env (make-capability-envelope
                   "owner"
                   allowed-caps: '(context-read)
                   denied-caps: '(host-egress)
                   egress-allowed?: #f))
             (ob (make-obligation "ob-e2e" title: "Verify Integrity" required-evidence-ids: '("ev-cas")))
             (det (make-deterministic-task-node
                   "det-cas-1" "ob-e2e" "ev-cas" 'funflow-check "digest"
                   effect-class: 'pure-cas read-footprint: '("a.ss") write-footprint: '()))
             (g0 (make-obligation-graph "g-e2e" cut: 1 obligations: (list ob) evidence: '() det-tasks: (list det)))
             (trunk (make-context-trunk "trunk-e2e" "ob-e2e" g0 env
                                        shared-context-cut: 1
                                        kv-prefix-digest: "sha256:e2e-zone1"))
             (actor (graph-make-task-actor "task-e2e-01" trunk '("scope-main") title: "E2E Closure"))
             (ctx (.o bind-these: #f
                      context-cut: 1
                      kv-prefix-digest: "sha256:e2e-zone1"
                      disclosure-depth: 'L3
                      has-active-defeater?: #f
                      sandbox-admitted?: #t
                      cedar-handoff-valid?: #t))
             (h1 (make-competing-hypothesis
                  "H1" "Build cache poisoning"
                  evidence-lineage: '("ev:hash")
                  contradictions: '("repro-ok")
                  open-questions: '("verify-signer?")))
             (h2 (make-competing-hypothesis
                  "H2" "Timestamp drift"
                  evidence-lineage: '("ev:mtime")
                  contradictions: '()
                  open-questions: '("epoch-env?")))
             (b1 (make-hypothesis-branch-proposal "B1" "Check signature"))
             (b2 (make-hypothesis-branch-proposal "B2" "Inspect archive"))
             (st0 (make-unified-driver-state
                   actor g0 ctx
                   competing-hypotheses: (list h1 h2)
                   branch-proposals: (list b1 b2)))
             (thin-pump
              (lambda (effs)
                (map (lambda (e)
                       (let ((k (.ref e 'kind)) (id (.ref e 'effect-id)))
                         (cond
                          ((eq? k 'cas-get) (make-cas-hit-receipt id "cas-evidence-ok"))
                          ((eq? k 'llm-complete) (make-llm-completed-receipt id (string-append "eval:" id)))
                          (else (make-effect-error-receipt id "unexpected")))))
                     effs)))
             (run (run-unified-driver-to-quiescence st0 thin-pump))
             (proj (project-driver-state->workbench-projection run))
             (pkg-ui (make-marlin-task-workbench-package proj disclosure-level: 'L3))
             (sess0 (marlin-ui-session-open
                     (vector pkg-ui #("gemini-2.5-pro" "scheme-closure" #() #()) #())))
             (step1 (marlin-ui-session-dispatch
                     (vector sess0 'scheme-action '#(task-switch #f))))
             (rendered (marlin-ui-session-project (vector-ref step1 0))))
        (check-equal? (.ref run 'final-phase) 'quiescent-fixed-point)
        (check-equal? (.ref run 'cas-hits-count) 1)
        (check-equal? (.ref run 'total-llm-calls) 4)
        (check-equal? (.ref proj 'visible-tab-ids) '(index "task-e2e-01"))
        (check-equal? (vector? rendered) #t)
        (check-equal? (vector-ref (vector-ref step1 1) 0) 'select-buffer)))

    (test-case "Multi-Actor Fleet + Cold-to-Warm CAS + Sibling Auto-Archive + Read-Only Peek renders in Marlin TUI (0 Rust edits)"
      (let* ((cas-table (make-hash-table))
             (pump (lambda (effs)
                     (map (lambda (e)
                            (let ((k (.ref e 'kind)) (id (.ref e 'effect-id)))
                              (cond
                               ((eq? k 'cas-get)
                                (let ((v (hash-get cas-table (.ref e 'key))))
                                  (if v (make-cas-hit-receipt id v) (make-cas-miss-receipt id))))
                               ((eq? k 'cas-put)
                                (hash-put! cas-table (.ref e 'key) (.ref e 'payload))
                                (make-cas-stored-receipt id))
                               ((eq? k 'llm-complete)
                                (make-llm-completed-receipt id (string-append "llm:" id)))
                               (else (make-effect-error-receipt id "err")))))
                          effs)))
             (env (make-capability-envelope "owner" allowed-caps: '(context-read) denied-caps: '(host-egress) egress-allowed?: #f))
             (ctx (.o bind-these: #f
                      context-cut: 2
                      kv-prefix-digest: "sha256:fleet-tui-zone1"
                      disclosure-depth: 'L3
                      has-active-defeater?: #f
                      sandbox-admitted?: #t
                      cedar-handoff-valid?: #t))
             (ob-p (make-obligation "ob-p" title: "Parent" required-evidence-ids: '("ev-p")))
             (det-p (make-deterministic-task-node "det-p" "ob-p" "ev-p" 'f-p "p" effect-class: 'pure-cas read-footprint: '("p") write-footprint: '()))
             (g-p (make-obligation-graph "g-p" cut: 2 obligations: (list ob-p) evidence: '() det-tasks: (list det-p)))
             (tr-p (make-context-trunk "tr-p" "ob-p" g-p env shared-context-cut: 2 kv-prefix-digest: "sha256:fleet-tui-zone1"))
             (p0 (graph-make-task-actor "task-fleet-parent" tr-p '("scope-main") title: "Fleet Parent"))
             (p-sq (.ref (graph-enter-task-actor-detour p0 'side-quest-active "sq-1") 'actor))
             (ob-s (make-obligation "ob-s" title: "Sibling" required-evidence-ids: '("ev-s")))
             (det-s (make-deterministic-task-node "det-s" "ob-s" "ev-s" 'f-s "s" effect-class: 'pure-cas read-footprint: '("s") write-footprint: '()))
             (g-s (make-obligation-graph "g-s" cut: 2 obligations: (list ob-s) evidence: '() det-tasks: (list det-s)))
             (fl0 (make-graph-fleet-state (list (make-unified-driver-state p-sq g-p ctx)) peek-set: '("task-fleet-sib")))
             (fl1 (fleet-spawn-sibling-from-detour fl0 "task-fleet-parent" "task-fleet-sib" "Fleet Sibling" g-s sibling-apex-id: "ob-s"))
             (p-st (car (.ref fl1 'active-drivers)))
             (s-st (cadr (.ref fl1 'active-drivers)))
             (p-blk (.ref (graph-enter-task-actor-detour (.ref p-st 'actor) 'blocker-active "blk-p") 'actor))
             (fl2 (make-graph-fleet-state
                   (list (make-unified-driver-state p-blk (.ref p-st 'obligation-graph) ctx) s-st)
                   peek-set: '("task-fleet-sib")))
             (run (run-graph-fleet-to-quiescence fl2 pump auto-archive?: #t))
             (proj (project-fleet->workbench-projection run))
             (pkg-ui (make-marlin-task-workbench-package proj disclosure-level: 'L3))
             (sess0 (marlin-ui-session-open
                     (vector pkg-ui #("gemini-2.5-pro" "fleet-workbench" #() #()) #())))
             (step1 (marlin-ui-session-dispatch
                     (vector sess0 'scheme-action '#(task-switch #f))))
             (rendered (marlin-ui-session-project (vector-ref step1 0))))
        (check-equal? (.ref run 'active-task-ids) '("task-fleet-parent"))
        (check-equal? (.ref run 'settled-task-ids) '("task-fleet-sib"))
        (check-equal? (.ref proj 'visible-tab-ids) '(index "task-fleet-parent" "task-fleet-sib"))
        (check-equal? (vector? rendered) #t)
        (check-equal? (vector-ref (vector-ref step1 1) 0) 'select-buffer)))

    (test-case "Round S2: Bitemporal defeater -> Blocker + L3 + Human Inbox -> Fleet CAS checkpoint/restore -> Marlin TUI (0 Rust edits)"
      (let* ((cas-table (make-hash-table))
             (pump (lambda (effs)
                     (map (lambda (e)
                            (let ((k (.ref e 'kind)) (id (.ref e 'effect-id)))
                              (cond
                               ((eq? k 'cas-put)
                                (hash-put! cas-table (.ref e 'key) (.ref e 'payload))
                                (make-cas-stored-receipt id))
                               ((eq? k 'cas-get)
                                (let ((v (hash-get cas-table (.ref e 'key))))
                                  (if v (make-cas-hit-receipt id v) (make-cas-miss-receipt id))))
                               (else (make-effect-error-receipt id "err")))))
                          effs)))
             (env (make-capability-envelope "owner" allowed-caps: '(context-read) denied-caps: '(host-egress) egress-allowed?: #f))
             (ctx (.o bind-these: #f
                      context-cut: 1
                      kv-prefix-digest: "sha256:s2-zone1"
                      disclosure-depth: 'L1
                      has-active-defeater?: #f
                      sandbox-admitted?: #t
                      cedar-handoff-valid?: #t))
             (ob (make-obligation "ob-s2" title: "Release Audit" required-evidence-ids: '("ev-s2")))
             (g0 (make-obligation-graph "g-s2" cut: 1 obligations: (list ob) evidence: '() det-tasks: '()))
             (tr0 (make-context-trunk "tr-s2" "ob-s2" g0 env shared-context-cut: 1 kv-prefix-digest: "sha256:s2-zone1"))
             (actor0 (graph-make-task-actor "task-s2-01" tr0 '("scope-main") title: "Release Gate"))
             (fl0 (make-graph-fleet-state (list (make-unified-driver-state actor0 g0 ctx))))
             (def-ev (.o bind-these: #f event-kind: 'defeater-discovered obligation-id: "ob-s2" defeater-id: "def-sig-01"))
             (fl-def (fleet-apply-bitemporal-event fl0 "task-s2-01" def-ev))
             ;; Persist full Fleet snapshot via CAS & restore cleanly
             (_put (pump (list (plan-fleet-cas-checkpoint fl-def "fleet:s2"))))
             (get-r (car (pump (list (make-cas-get-effect "get-s2" "fleet:s2")))))
             (fl-restored (restore-fleet-from-cas-receipt get-r))
             (proj (project-fleet->workbench-projection fl-restored))
             (tab (car (.ref proj 'dynamic-tabs)))
             (pkg-ui (make-marlin-task-workbench-package proj disclosure-level: 'L3))
             (sess0 (marlin-ui-session-open
                     (vector pkg-ui #("gemini-2.5-pro" "s2-inbox" #() #()) #())))
             (rendered (marlin-ui-session-project sess0)))
        (check-equal? (.ref tab 'detour-mode) 'blocker-active)
        (check-equal? (.ref tab 'active-defeaters-count) 1)
        (check-equal? (length (.ref tab 'human-inbox)) 1)
        (check-equal? (vector? rendered) #t)))

    (test-case "Round S5: Interactive Scheme TaskWorkbenchController live-reprojects disclosure and Detour 4-exit sibling tabs (0 Rust edits)"
      (let* ((proj0 (.o bind-these: #f
                        pinned-tab: 'index
                        dynamic-tabs:
                        (list (.o bind-these: #f
                                  id: "task-ctrl-01"
                                  label: "Interactive Controller"
                                  status: 'in-progress
                                  mode: 'active
                                  detour-mode: 'mainline
                                  detour-node-id: #f
                                  worktree: "wt/ctrl"
                                  checklist-done: 1
                                  checklist-total: 2
                                  competing-hypotheses: '()))
                        archived-index: '()
                        visible-tab-ids: '(index "task-ctrl-01")))
             (c0 (make-marlin-task-workbench-controller proj0 disclosure-level: 'L1))
             (c1 (marlin-task-workbench-controller-apply-action c0 'task-drill-l3))
             (c2 (marlin-task-workbench-controller-apply-action c1 'task-detour-blocker "task-ctrl-01" "blk-live"))
             (c3 (marlin-task-workbench-controller-apply-action c2 'task-detour-exit-sibling "task-ctrl-01" "task-ctrl-sib"))
             (vec3 (marlin-task-workbench-controller-project c3)))
        (check-equal? (.ref c1 'disclosure-level) 'L3)
        (check-equal? (.ref (car (.ref (.ref c2 'projection) 'dynamic-tabs)) 'detour-mode) 'blocker-active)
        (check-equal? (.ref (.ref c3 'projection) 'visible-tab-ids) '(index "task-ctrl-01" "task-ctrl-sib"))
        (check-equal? (vector? vec3) #t)))))
