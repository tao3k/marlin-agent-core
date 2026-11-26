;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Task-as-Tab Workbench Package (RFC 40.183 §7.6):
;;; Projects Durable `WorkbenchTabProjection` (Left Pinned `Index` + Right Dynamic
;;; `Active Task Tabs` / `Peek` tabs + Progressive `L0..L3` Disclosure commands)
;;; into a standard checked Marlin `UiScope` via `UiPackage.` and `marlin-ui-layout-with-tab-bar`.

(import (only-in :clan/poo/object .o .ref .slot? .get object?)
        (only-in :clan/poo/type define-type Enum validate)
        (only-in :std/list/list filter take)
        (only-in "../ui-centaur-tabs/funcs.ss"
                 marlin-ui-task-tab-identity marlin-ui-task-tab-bar-for)
        (only-in "../ui-elements/funcs.ss" ui-rows ui-text)
        (only-in "../ui-layout/config.ss" marlin-ui-layout-with-tab-bar)
        (only-in "../ui-meow/interface.ss" define-key-bindings meow-normal)
        (only-in "./package.ss" UiPackage. make-ui-package))

(export TaskDisclosureLevel
        TaskDetourMode
        TaskWorkbenchKeymap.
        marlin-task-workbench-commands
        marlin-task-workbench-buffers
        marlin-task-workbench-peek-set-add
        marlin-task-workbench-peek-set-remove
        marlin-task-workbench-next-disclosure
        marlin-task-workbench-apply-detour-action
        make-marlin-task-workbench-package)

(define-type TaskDisclosureLevel (Enum L0 L1 L2 L3))
(define-type TaskDetourMode (Enum mainline blocker-active side-quest-active))

(def (opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

;;; Doom/Meow-style Task Workbench keybindings under `SPC o` (Org/Task workbench),
;;; strictly disjoint from `UiMeowScope` (`SPC x`, `SPC b`, `SPC t`, `SPC h`, `SPC r`, `SPC m`).
(define-key-bindings TaskWorkbenchKeymap.
  (meow-normal
    ((SPC o i) task-index "Open pinned Task Index")
    ((SPC o t) task-switch "Switch active Task tab")
    ((SPC o p) task-peek "Peek archived Task read-only")
    ((SPC o r) task-reopen "Reopen archived Task into active tabs")
    ((SPC o #\0) task-drill-l0 "Fold Org disclosure to L0 Skeleton")
    ((SPC o #\1) task-drill-l1 "Expand Org disclosure to L1 Checklist")
    ((SPC o #\2) task-drill-l2 "Expand Org disclosure to L2 Synthesis Body")
    ((SPC o #\3) task-drill-l3 "Expand Org disclosure to L3 Full Evidence")
    ((SPC o d b) task-detour-blocker "Enter Blocker-Active Scoped Detour")
    ((SPC o d s) task-detour-sidequest "Enter Side-Quest Scoped Detour")
    ((SPC o d r) task-detour-exit-return "Exit Detour: Return to Origin")
    ((SPC o d a) task-detour-exit-absorb "Exit Detour: Absorb and Continue")
    ((SPC o d n) task-detour-exit-sibling "Exit Detour: Spawn Sibling Task")
    ((SPC o d x) task-detour-exit-abort "Exit Detour: Abort Detour")
    ((SPC o h a) task-human-approve "Approve human gate and resolve active defeater")
    ((SPC o h b) task-propose-branch "Propose bounded hypothesis branch for HxB evaluation")
    ((SPC o h p) task-propagate-defeater "Propagate shared-premise defeater across live Fleet")
    ((SPC o f s) task-fleet-step "Advance live GraphEngine Fleet one pure step")
    ((SPC o f c) task-fleet-checkpoint "Emit Fleet CAS checkpoint effect")))

(def (marlin-task-workbench-commands)
  '(("task-index" "Focus pinned Task Index buffer" none select-buffer "index")
    ("task-switch" "Switch among visible active Task tabs" none scheme-action "task-switch")
    ("task-peek" "Peek an archived Task in read-only mode" optional scheme-action "task-peek")
    ("task-reopen" "Durably reopen an archived Task into active tabs" optional scheme-action "task-reopen")
    ("task-drill-l0" "Set progressive Org disclosure to L0 Skeleton" none scheme-action "L0")
    ("task-drill-l1" "Set progressive Org disclosure to L1 Contract & Checklist" none scheme-action "L1")
    ("task-drill-l2" "Set progressive Org disclosure to L2 Synthesis Body" none scheme-action "L2")
    ("task-drill-l3" "Set progressive Org disclosure to L3 Full Evidence" none scheme-action "L3")
    ("task-detour-blocker" "Enter Blocker-Active Scoped Detour on focused Task" optional scheme-action "task-detour-blocker")
    ("task-detour-sidequest" "Enter Side-Quest Scoped Detour on focused Task" optional scheme-action "task-detour-sidequest")
    ("task-detour-exit-return" "Exit active Scoped Detour via return-to-origin" none scheme-action "task-detour-exit-return")
    ("task-detour-exit-absorb" "Exit active Scoped Detour via absorb-and-continue" none scheme-action "task-detour-exit-absorb")
    ("task-detour-exit-sibling" "Exit active Scoped Detour by spawning a sibling Task tab" optional scheme-action "task-detour-exit-sibling")
    ("task-detour-exit-abort" "Exit active Scoped Detour via abort-detour" none scheme-action "task-detour-exit-abort")
    ("task-human-approve" "Approve human gate and resolve active defeater on focused Task" optional scheme-action "task-human-approve")
    ("task-propose-branch" "Append bounded HypothesisBranchProposal to focused Task for HxB evaluation" optional scheme-action "task-propose-branch")
    ("task-propagate-defeater" "Propagate shared-premise defeater across dependent Fleet actors" optional scheme-action "task-propagate-defeater")
    ("task-fleet-step" "Advance live GraphEngine Fleet one pure ASCENT/FunFlow step" none scheme-action "task-fleet-step")
    ("task-fleet-checkpoint" "Emit canonical UTF-8 Fleet CAS checkpoint effect" optional scheme-action "task-fleet-checkpoint")))

(def (task-summary-line tab-obj)
  (let ((tid (symbol->string (marlin-ui-task-tab-identity (.ref tab-obj 'id))))
        (label (opt-ref tab-obj 'label ""))
        (status (symbol->string (opt-ref tab-obj 'status 'todo)))
        (mode (symbol->string (opt-ref tab-obj 'mode 'active)))
        (detour (opt-ref tab-obj 'detour-mode 'mainline))
        (phase (opt-ref tab-obj 'driver-phase #f))
        (inbox (opt-ref tab-obj 'human-inbox '()))
        (defeaters (opt-ref tab-obj 'active-defeaters-count 0))
        (wt (opt-ref tab-obj 'worktree "main"))
        (done-c (opt-ref tab-obj 'checklist-done 0))
        (total-c (opt-ref tab-obj 'checklist-total 0)))
    (string-append tid "  [" status "/" mode
                   (if (eq? detour 'mainline)
                     ""
                     (string-append "/" (symbol->string detour)))
                   (if phase
                     (string-append " · " (symbol->string phase))
                     "")
                   (if (> (length inbox) 0)
                     (string-append " · INBOX:" (number->string (length inbox)))
                     "")
                   (if (> defeaters 0)
                     (string-append " · DEFEATERS:" (number->string defeaters))
                     "")
                   "]  " label
                   "  (" (number->string done-c) "/" (number->string total-c) ")  wt:" wt)))

(def (archived-summary-line rec)
  (let ((tid (opt-ref rec 'task-id "unknown"))
        (title (opt-ref rec 'title ""))
        (wt (opt-ref rec 'worktree "main")))
    (string-append tid "  [ARCHIVED]  " title "  wt:" wt)))

(def (make-index-buffer-entry projection disclosure-level)
  (let* ((dynamic (opt-ref projection 'dynamic-tabs '()))
         (archived (opt-ref projection 'archived-index '()))
         (header (string-append "Task Index · Active: " (number->string (length dynamic))
                                " · Archived: " (number->string (length archived))
                                " · Disclosure: " (symbol->string disclosure-level)))
         (lines (append
                 (list (cons 'accent header))
                 (if (null? dynamic)
                   (list (cons 'muted "  (no active tasks)"))
                   (map (lambda (t) (cons 'normal (string-append "  " (task-summary-line t))))
                        (take dynamic (min 6 (length dynamic)))))
                 (map (lambda (a) (cons 'muted (string-append "  " (archived-summary-line a))))
                      (take archived (min 4 (length archived)))))))
    (list 'index
          "Pinned Plan-A Task Index"
          (list "*Index*" (ui-rows (list (ui-text lines))))
          #f)))

(def (detour-badge-line detour-mode detour-node-id)
  (cond
   ((eq? detour-mode 'blocker-active)
    (cons 'warning
          (string-append "Detour: [BLOCKER-ACTIVE]"
                         (if detour-node-id (string-append " node:" detour-node-id) "")
                         " · Settlement halted (SPC o d r/a/n/x to exit)")))
   ((eq? detour-mode 'side-quest-active)
    (cons 'info
          (string-append "Detour: [SIDE-QUEST]"
                         (if detour-node-id (string-append " node:" detour-node-id) "")
                         " · Exploratory scope (SPC o d r/a/n/x to exit)")))
   (else
    (cons 'muted "Detour: [MAINLINE] · SPC o d b Blocker · SPC o d s Side-Quest"))))

(def (competing-hypothesis-lines tab-obj disclosure-level)
  (let ((hyps (opt-ref tab-obj 'competing-hypotheses '())))
    (if (and (pair? hyps) (memq disclosure-level '(L2 L3)))
      (cons
       (cons 'accent
             (string-append "Competing Hypotheses (" (number->string (length hyps))
                            " preserved · non-authorizing):"))
       (map (lambda (h)
              (let ((hid (opt-ref h 'hypothesis-id "H?"))
                    (claim (opt-ref h 'claim ""))
                    (contras (opt-ref h 'contradictions '()))
                    (opens (opt-ref h 'open-questions '()))
                    (obs-b (opt-ref h 'observed-branches #f))
                    (tot-b (opt-ref h 'total-branches #f)))
                (cons 'normal
                      (string-append "  • [" hid "] " claim
                                     (if (and obs-b tot-b)
                                       (string-append " [observed:" (number->string obs-b)
                                                      "/" (number->string tot-b) "]")
                                       "")
                                     " (contradictions:" (number->string (length contras))
                                     ", open:" (number->string (length opens)) ")"))))
            (take hyps (min 4 (length hyps)))))
      '())))

(def (human-inbox-lines tab-obj disclosure-level)
  (let ((inbox (opt-ref tab-obj 'human-inbox '()))
        (defeaters (opt-ref tab-obj 'active-defeaters-count 0))
        (rep-attempts (opt-ref tab-obj 'repair-attempts 0))
        (rep-budget (opt-ref tab-obj 'repair-budget #f)))
    (if (and (memq disclosure-level '(L1 L2 L3))
             (or (pair? inbox) (> defeaters 0) (> rep-attempts 0)))
      (cons
       (cons 'warning
             (string-append "Human Inbox & Diagnostics · Items:" (number->string (length inbox))
                            " · Active Defeaters:" (number->string defeaters)
                            (if rep-budget
                              (string-append " · Repair Budget:" (number->string rep-attempts)
                                             "/" (number->string rep-budget))
                              "")))
       (map (lambda (item)
              (let ((k (opt-ref item 'kind 'diagnostic))
                    (detail (opt-ref item 'detail "")))
                (cons 'warning
                      (string-append "  ! [" (symbol->string k) "] " detail))))
            (take inbox (min 4 (length inbox)))))
      '())))

(def (make-task-buffer-entry tab-obj disclosure-level)
  (let* ((id (marlin-ui-task-tab-identity (.ref tab-obj 'id)))
         (label (opt-ref tab-obj 'label (symbol->string id)))
         (mode (opt-ref tab-obj 'mode 'active))
         (status (opt-ref tab-obj 'status 'todo))
         (detour (opt-ref tab-obj 'detour-mode 'mainline))
         (detour-node (opt-ref tab-obj 'detour-node-id #f))
         (phase (opt-ref tab-obj 'driver-phase #f))
         (pending-effs (opt-ref tab-obj 'pending-effects-count 0))
         (wt (opt-ref tab-obj 'worktree "main"))
         (done-c (opt-ref tab-obj 'checklist-done 0))
         (total-c (opt-ref tab-obj 'checklist-total 0))
         (title (string-append "*Task: " label "*"))
         (lines
          (append
           (list (cons 'accent (string-append "Task: " (symbol->string id) " · " label))
                 (cons 'info (string-append "Status: " (symbol->string status)
                                            " · Mode: " (symbol->string mode)
                                            (if phase
                                              (string-append " · Phase: " (symbol->string phase)
                                                             " (effects:" (number->string pending-effs) ")")
                                              "")
                                            " · Worktree: " wt))
                 (detour-badge-line detour detour-node)
                 (cons 'normal (string-append "Checklist: [" (number->string done-c)
                                              "/" (number->string total-c) "]"
                                              " · Disclosure: " (symbol->string disclosure-level))))
           (human-inbox-lines tab-obj disclosure-level)
           (competing-hypothesis-lines tab-obj disclosure-level)
           (list (cons 'muted (if (eq? mode 'peek-readonly)
                                "Read-only peek mode (SPC o r to reopen durably)"
                                "SPC o i Index · SPC o 0..3 Disclosure L0..L3 · SPC o d Detour"))))))
    (list id
          (string-append "Task buffer: " label)
          (list title (ui-rows (list (ui-text lines))))
          #f)))

;;; Produce bounded static buffer declarations for `make-ui-package` (max 8 buffers:
;;; 1 pinned `index` + up to 7 visible dynamic tasks).
(def (marlin-task-workbench-buffers projection (disclosure-level 'L1))
  (validate TaskDisclosureLevel disclosure-level)
  (let* ((dynamic (opt-ref projection 'dynamic-tabs '()))
         (bounded-dynamic (take dynamic (min 7 (length dynamic)))))
    (cons (make-index-buffer-entry projection disclosure-level)
          (map (lambda (t) (make-task-buffer-entry t disclosure-level))
               bounded-dynamic))))

;;; Pure functional state transitions (caller owns next state).
(def (marlin-task-workbench-peek-set-add peek-set task-id)
  (if (member task-id peek-set)
    peek-set
    (cons task-id peek-set)))

(def (marlin-task-workbench-peek-set-remove peek-set task-id)
  (filter (lambda (id) (not (equal? id task-id))) peek-set))

(def (marlin-task-workbench-next-disclosure _current-level target-level)
  (validate TaskDisclosureLevel target-level))

(def (clone-tab-with-detour tab-obj new-detour new-node)
  (.o bind-these: #f
      id: (.ref tab-obj 'id)
      label: (opt-ref tab-obj 'label "")
      status: (opt-ref tab-obj 'status 'in-progress)
      mode: (opt-ref tab-obj 'mode 'active)
      detour-mode: new-detour
      detour-node-id: new-node
      worktree: (opt-ref tab-obj 'worktree "main")
      checklist-done: (opt-ref tab-obj 'checklist-done 0)
      checklist-total: (opt-ref tab-obj 'checklist-total 0)
      competing-hypotheses: (opt-ref tab-obj 'competing-hypotheses '())))

;;; Pure Scheme projection transition for Design 165 Scoped Detour 4-Exit actions
;;; (enter blocker/side-quest, or exit via return-to-origin / absorb-and-continue /
;;; spawn-sibling-task / abort-detour) with ZERO Rust UI code touched!
(def (marlin-task-workbench-apply-detour-action projection task-id action (arg #f))
  (let* ((dynamic (opt-ref projection 'dynamic-tabs '()))
         (archived (opt-ref projection 'archived-index '()))
         (sibling-id (if (and (string? arg) (> (string-length arg) 0))
                       arg
                       (string-append task-id "-sibling")))
         (updated-dynamic
          (let loop ((tabs dynamic) (acc '()) (spawned #f))
            (if (null? tabs)
              (let ((base (reverse acc)))
                (if spawned (append base (list spawned)) base))
              (let ((t (car tabs)))
                (if (equal? (.ref t 'id) task-id)
                  (case action
                    ((task-detour-blocker)
                     (loop (cdr tabs)
                           (cons (clone-tab-with-detour
                                  t 'blocker-active (or arg (string-append "blk-" task-id)))
                                 acc)
                           spawned))
                    ((task-detour-sidequest)
                     (loop (cdr tabs)
                           (cons (clone-tab-with-detour
                                  t 'side-quest-active (or arg (string-append "sq-" task-id)))
                                 acc)
                           spawned))
                    ((task-detour-exit-return task-detour-exit-absorb task-detour-exit-abort)
                     (loop (cdr tabs)
                           (cons (clone-tab-with-detour t 'mainline #f) acc)
                           spawned))
                    ((task-detour-exit-sibling)
                     (let ((sib (.o bind-these: #f
                                    id: sibling-id
                                    label: (string-append "Sibling of " (opt-ref t 'label task-id))
                                    status: 'in-progress
                                    mode: 'active
                                    detour-mode: 'mainline
                                    detour-node-id: #f
                                    worktree: (opt-ref t 'worktree "main")
                                    checklist-done: 0
                                    checklist-total: 1
                                    competing-hypotheses: (opt-ref t 'competing-hypotheses '()))))
                       (loop (cdr tabs)
                             (cons (clone-tab-with-detour t 'mainline #f) acc)
                             sib)))
                    (else (error "unknown task-workbench detour action" action)))
                  (loop (cdr tabs) (cons t acc) spawned)))))))
    (.o bind-these: #f
        pinned-tab: (opt-ref projection 'pinned-tab 'index)
        dynamic-tabs: updated-dynamic
        archived-index: archived
        visible-tab-ids: (cons 'index (map (lambda (t) (.ref t 'id)) updated-dynamic)))))

;;; Compose a full checked Marlin `UiScope` bound to a live `WorkbenchTabProjection`.
(def (make-marlin-task-workbench-package projection
                                         disclosure-level: (disclosure-level 'L1)
                                         nerd-icons?: (nerd-icons? #t))
  (validate TaskDisclosureLevel disclosure-level)
  (make-ui-package
    (.o (:: @ UiPackage.)
        buffers: (marlin-task-workbench-buffers projection disclosure-level)
        commands: (marlin-task-workbench-commands)
        keymap: (.get TaskWorkbenchKeymap. bindings)
        layout: (marlin-ui-layout-with-tab-bar
                  (marlin-ui-task-tab-bar-for projection nerd-icons?: nerd-icons?)))))
