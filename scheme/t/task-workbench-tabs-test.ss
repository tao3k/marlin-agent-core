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
                 marlin-ui-session-dispatch))

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
        (check-equal? (vector-ref (vector-ref step-sib 1) 0) 'select-buffer)))))
