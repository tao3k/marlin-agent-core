;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Authoritative Major Mode hierarchy, orthogonal Minor Modes, Polymode Inner
;;; Modes, Polymode host->inner rules, and Buffer-to-Mode routing table.
(import (only-in :clan/poo/object .o))
(export DefaultMajorModes
        DefaultMinorModes
        DefaultInnerModes
        DefaultPolymodes
        DefaultBufferModeRules)

;; 1. Major Mode Inheritance Hierarchy (define-derived-mode style)
(def DefaultMajorModes
  (list
    (.o id: 'fundamental-mode
        title: "Fundamental"
        parent: #f
        read-only: #f
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: '()
        default-minor-modes: '(meow-normal-mode)
        modeline-segments: '(key-mode buffer-name major-mode workspace-path activity)
        new-buffer-actions: '("cmd:new-chat-buffer" "cmd:switch-buffer")
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-special-mode
        title: "Special"
        parent: 'fundamental-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "q" command: "cmd:win-delete" label: "Quit window" exit: #t)
                        (.o key: "g" command: "cmd:buffer-refresh" label: "Refresh buffer" exit: #t)
                        (.o key: "j" command: "cmd:cursor-down" label: "Next item" exit: #f)
                        (.o key: "k" command: "cmd:cursor-up" label: "Previous item" exit: #f))
        default-minor-modes: '(meow-normal-mode read-only-mode)
        modeline-segments: '(key-mode buffer-name major-mode workspace-path activity)
        new-buffer-actions: '("cmd:switch-buffer")
        enter-hooks: '(enter-read-only)
        exit-hooks: '())
    (.o id: 'marlin-chat-mode
        title: "Marlin-Chat"
        parent: 'fundamental-mode
        read-only: #f
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "i" command: "cmd:insert" label: "Focus prompt (Insert)" exit: #t)
                        (.o key: "C-c C-n" command: "cmd:new-chat-buffer" label: "New LLM Chat buffer" exit: #t)
                        (.o key: "C-c C-d" command: "cmd:toggle-deep-think" label: "Toggle DeepThink" exit: #t)
                        (.o key: "C-c C-m" command: "buffer:*Models*" label: "Switch LLM model" exit: #t))
        default-minor-modes: '(meow-normal-mode llm-inline-assist-mode aitia-audit-minor-mode)
        modeline-segments: '(key-mode buffer-name major-mode polymode-inner deep-think composer-dock model-label session-id activity gate-badge)
        new-buffer-actions: '("cmd:new-chat-buffer" "cmd:clone-window" "cmd:magit-status")
        enter-hooks: '(attach-llm-session)
        exit-hooks: '())
    (.o id: 'marlin-room-mode
        title: "Agent-Room"
        parent: 'fundamental-mode
        read-only: #f
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "i" command: "cmd:insert" label: "Compose room message" exit: #t)
                        (.o key: "C-c C-t" command: "cmd:room-add-task" label: "Add room task" exit: #t)
                        (.o key: "C-c C-a" command: "cmd:room-list-agents" label: "Inspect active agents" exit: #t))
        default-minor-modes: '(meow-normal-mode llm-inline-assist-mode)
        modeline-segments: '(key-mode buffer-name major-mode polymode-inner room-stats activity gate-badge)
        new-buffer-actions: '("cmd:new-chat-buffer" "cmd:clone-window")
        enter-hooks: '(attach-room-stream)
        exit-hooks: '())
    (.o id: 'magit-mode
        title: "Magit"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "g" command: "cmd:magit-refresh" label: "Refresh & re-run Aitia gRPC" exit: #t)
                        (.o key: "?" command: "hydra:magit:?" label: "Magit dispatch" exit: #f)
                        (.o key: "h" command: "hydra:magit:?" label: "Magit dispatch" exit: #f)
                        (.o key: "TAB" command: "cmd:magit-toggle-fold" label: "Toggle section fold" exit: #f)
                        (.o key: "1" command: "cmd:magit-level-1" label: "Visibility Level 1" exit: #f)
                        (.o key: "2" command: "cmd:magit-level-2" label: "Visibility Level 2" exit: #f)
                        (.o key: "3" command: "cmd:magit-level-3" label: "Visibility Level 3" exit: #f)
                        (.o key: "4" command: "cmd:magit-level-4" label: "Visibility Level 4" exit: #f)
                        (.o key: "n" command: "cmd:magit-next-sibling" label: "Next sibling section" exit: #f)
                        (.o key: "p" command: "cmd:magit-prev-sibling" label: "Previous sibling section" exit: #f)
                        (.o key: "^" command: "cmd:magit-parent-section" label: "Jump to parent section" exit: #f)
                        (.o key: "RET" command: "cmd:magit-visit-thing" label: "Visit file at hunk line" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode aitia-audit-minor-mode)
        modeline-segments: '(key-mode buffer-name major-mode polymode-inner aitia-status branch-ref activity gate-badge)
        new-buffer-actions: '("cmd:magit-refresh" "cmd:magit-ai-audit" "cmd:clone-window")
        enter-hooks: '(refresh-magit-and-aitia)
        exit-hooks: '())
    (.o id: 'magit-status-mode
        title: "Magit-Status"
        parent: 'magit-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "s" command: "cmd:magit-stage" label: "Stage hunk or file" exit: #t)
                        (.o key: "u" command: "cmd:magit-unstage" label: "Unstage hunk or file" exit: #t)
                        (.o key: "x" command: "cmd:magit-discard" label: "Discard hunk" exit: #t)
                        (.o key: "c" command: "hydra:magit:c" label: "Commit / jj describe" exit: #f)
                        (.o key: "d" command: "hydra:magit:d" label: "Diff & Aitia/AI audit" exit: #f)
                        (.o key: "b" command: "hydra:magit:b" label: "Branch / jj bookmark" exit: #f)
                        (.o key: "f" command: "hydra:magit:f" label: "Fetch" exit: #f)
                        (.o key: "F" command: "hydra:magit:F" label: "Pull" exit: #f)
                        (.o key: "P" command: "hydra:magit:P" label: "Push" exit: #f)
                        (.o key: "l" command: "hydra:magit:l" label: "Log / jj op log" exit: #f)
                        (.o key: "r" command: "hydra:magit:r" label: "Rebase / squash / AI split" exit: #f)
                        (.o key: "z" command: "hydra:magit:z" label: "Stash / jj undo" exit: #f)
                        (.o key: "X" command: "hydra:magit:X" label: "Reset" exit: #f)
                        (.o key: "A" command: "hydra:magit:A" label: "Cherry-pick" exit: #f)
                        (.o key: "V" command: "hydra:magit:V" label: "Revert" exit: #f)
                        (.o key: "m" command: "hydra:magit:m" label: "Merge / AI resolve" exit: #f)
                        (.o key: "a" command: "cmd:magit-ai-audit" label: "AI Code Audit (50/50 split)" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode aitia-audit-minor-mode)
        modeline-segments: '(key-mode buffer-name major-mode polymode-inner aitia-status branch-ref activity gate-badge)
        new-buffer-actions: '("cmd:magit-refresh" "cmd:magit-ai-audit" "cmd:clone-window")
        enter-hooks: '(refresh-magit-and-aitia)
        exit-hooks: '())
    (.o id: 'magit-diff-mode
        title: "Magit-Diff"
        parent: 'magit-mode
        read-only: #t
        display-action: 'split-right
        dedicated: #f
        local-keymap: (list
                        (.o key: "s" command: "cmd:magit-stage" label: "Stage hunk" exit: #t)
                        (.o key: "u" command: "cmd:magit-unstage" label: "Unstage hunk" exit: #t)
                        (.o key: "a" command: "cmd:magit-ai-audit" label: "AI Code Audit hunk" exit: #t)
                        (.o key: "SPC" command: "cmd:magit-audit-toggle" label: "Mark hunk audited" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode aitia-audit-minor-mode)
        modeline-segments: '(key-mode buffer-name major-mode aitia-status activity gate-badge)
        new-buffer-actions: '("cmd:magit-ai-audit")
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'magit-log-mode
        title: "Magit-Log"
        parent: 'magit-mode
        read-only: #t
        display-action: 'split-right
        dedicated: #f
        local-keymap: (list
                        (.o key: "o" command: "cmd:magit-toggle-oplog" label: "Toggle jj Op Log" exit: #t)
                        (.o key: "A" command: "hydra:magit:A" label: "Cherry-pick revision" exit: #f)
                        (.o key: "V" command: "hydra:magit:V" label: "Revert revision" exit: #f))
        default-minor-modes: '(meow-normal-mode read-only-mode)
        modeline-segments: '(key-mode buffer-name major-mode branch-ref activity gate-badge)
        new-buffer-actions: '("cmd:magit-refresh")
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-dashboard-mode
        title: "Bento-Dashboard"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "r" command: "cmd:reload-theme" label: "Reload dashboard widgets" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode)
        modeline-segments: '(key-mode buffer-name major-mode workspace-path dashboard-preset version)
        new-buffer-actions: '("cmd:new-chat-buffer" "cmd:magit-status")
        enter-hooks: '(enable-dashboard-clock)
        exit-hooks: '(disable-dashboard-clock))
    (.o id: 'marlin-workflow-mode
        title: "Workflow-Canvas"
        parent: 'marlin-special-mode
        read-only: #f
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "r" command: "cmd:workflow-fit" label: "Fit BPMN canvas" exit: #t))
        default-minor-modes: '(meow-normal-mode)
        modeline-segments: '(key-mode buffer-name major-mode workspace-path activity gate-badge)
        new-buffer-actions: '("cmd:clone-window")
        enter-hooks: '(show-webisland)
        exit-hooks: '(hide-webisland))
    (.o id: 'marlin-tabulated-mode
        title: "Tabulated-List"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "RET" command: "cmd:table-select" label: "Open selected row" exit: #t)
                        (.o key: "d" command: "cmd:ibuffer-mark-cursor" label: "Mark cursor buffer for delete" exit: #f)
                        (.o key: "u" command: "cmd:ibuffer-unmark-cursor" label: "Unmark cursor buffer" exit: #f)
                        (.o key: "U" command: "cmd:ibuffer-unmark-all" label: "Unmark all buffers" exit: #t)
                        (.o key: "x" command: "cmd:ibuffer-kill-marked" label: "Kill marked buffers" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode)
        modeline-segments: '(key-mode buffer-name major-mode row-count activity gate-badge)
        new-buffer-actions: '("cmd:switch-buffer")
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-preview-mode
        title: "Preview-Mode"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'split-right
        dedicated: #f
        local-keymap: (list
                        (.o key: "i" command: "cmd:consult-imenu" label: "Jump to symbol (ASP Imenu)" exit: #t)
                        (.o key: "/" command: "cmd:consult-line" label: "Search lines in file" exit: #t)
                        (.o key: "m" command: "cmd:bookmark-set" label: "Bookmark current file:line" exit: #t)
                        (.o key: "a" command: "cmd:magit-ai-audit" label: "AI audit previewed file" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode aitia-audit-minor-mode)
        modeline-segments: '(key-mode buffer-name major-mode workspace-path gate-badge)
        new-buffer-actions: '("cmd:clone-window")
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-help-mode
        title: "Help"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'popup-bottom
        dedicated: #t
        local-keymap: '()
        default-minor-modes: '(meow-normal-mode read-only-mode dedicated-window-mode)
        modeline-segments: '(key-mode buffer-name major-mode gate-badge)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-messages-mode
        title: "Messages"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'popup-bottom
        dedicated: #t
        local-keymap: '()
        default-minor-modes: '(meow-normal-mode read-only-mode dedicated-window-mode)
        modeline-segments: '(key-mode buffer-name major-mode gate-badge)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-settings-mode
        title: "Customize"
        parent: 'marlin-special-mode
        read-only: #f
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "C-c C-c" command: "cmd:reload-theme" label: "Apply customize settings" exit: #t))
        default-minor-modes: '(meow-normal-mode)
        modeline-segments: '(key-mode buffer-name major-mode gate-badge)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-packages-mode
        title: "Package-Menu"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "U" command: "cmd:refresh-packages" label: "Refresh Marketplace index" exit: #t))
        default-minor-modes: '(meow-normal-mode read-only-mode)
        modeline-segments: '(key-mode buffer-name major-mode activity gate-badge)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())))

;; 2. Orthogonal Minor Modes (composable on any buffer or globally)
(def DefaultMinorModes
  (list
    (.o id: 'meow-normal-mode
        title: "Meow Normal"
        lighter: "NORMAL"
        priority: 100
        local-keymap: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'meow-insert-mode
        title: "Meow Insert"
        lighter: "INSERT"
        priority: 110
        local-keymap: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'aitia-audit-minor-mode
        title: "Lambda Aitia Audit"
        lighter: "λAitia"
        priority: 80
        local-keymap: (list
                        (.o key: "C-c g" command: "cmd:magit-ai-audit" label: "AI + Aitia Code Audit" exit: #t)
                        (.o key: "C-c M-g" command: "hydra:magit:d" label: "Aitia Diff Dispatch" exit: #f))
        enter-hooks: '(probe-aitia-sidecar)
        exit-hooks: '())
    (.o id: 'llm-inline-assist-mode
        title: "LLM Inline Assist"
        lighter: "AI"
        priority: 60
        local-keymap: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'zen-writeroom-mode
        title: "Zen Writeroom"
        lighter: "Zen"
        priority: 50
        local-keymap: '()
        enter-hooks: '(hide-chrome-zen)
        exit-hooks: '(restore-chrome-zen))
    (.o id: 'liquid-glass-mode
        title: "Liquid Glass"
        lighter: "Glass"
        priority: 40
        local-keymap: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'dedicated-window-mode
        title: "Dedicated Window"
        lighter: "Pin"
        priority: 30
        local-keymap: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'read-only-mode
        title: "Read-Only"
        lighter: "RO"
        priority: 20
        local-keymap: '()
        enter-hooks: '()
        exit-hooks: '())))

;; 3. Modern Polymode Inner Modes (zero indirect-buffer, span-driven)
(def DefaultInnerModes
  (list
    (.o id: 'prog-code-inner-mode
        title: "Code-Span"
        parent: 'fundamental-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "y" command: "cmd:span-copy-code" label: "Copy code block" exit: #t)
                        (.o key: "RET" command: "cmd:span-open-preview" label: "Open code in *Preview* split" exit: #t)
                        (.o key: "a" command: "cmd:span-ai-explain" label: "AI explain / audit code span" exit: #t))
        default-minor-modes: '()
        modeline-segments: '(polymode-inner)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'magit-diff-inner-mode
        title: "Diff-Hunk"
        parent: 'magit-diff-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "s" command: "cmd:magit-stage" label: "Stage diff hunk" exit: #t)
                        (.o key: "u" command: "cmd:magit-unstage" label: "Unstage diff hunk" exit: #t)
                        (.o key: "x" command: "cmd:magit-discard" label: "Discard diff hunk" exit: #t)
                        (.o key: "a" command: "cmd:magit-ai-audit" label: "AI + Aitia Audit hunk (50/50 split)" exit: #t)
                        (.o key: "SPC" command: "cmd:magit-audit-toggle" label: "Toggle hunk audited" exit: #t)
                        (.o key: "RET" command: "cmd:magit-visit-thing" label: "Visit file at hunk line" exit: #t))
        default-minor-modes: '(aitia-audit-minor-mode)
        modeline-segments: '(polymode-inner aitia-status)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-gate-inner-mode
        title: "Approval-Gate"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'reuse-window
        dedicated: #f
        local-keymap: (list
                        (.o key: "y" command: "cmd:gate-approve" label: "Approve tool/action gate" exit: #t)
                        (.o key: "n" command: "cmd:gate-reject" label: "Reject tool/action gate" exit: #t)
                        (.o key: "i" command: "cmd:review-gate" label: "Inspect gate details in Minibuffer" exit: #t))
        default-minor-modes: '()
        modeline-segments: '(polymode-inner)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())
    (.o id: 'marlin-artifact-inner-mode
        title: "Artifact-Span"
        parent: 'marlin-special-mode
        read-only: #t
        display-action: 'split-right
        dedicated: #f
        local-keymap: (list
                        (.o key: "RET" command: "cmd:artifact-open-stage" label: "Open artifact in Stage split" exit: #t)
                        (.o key: "y" command: "cmd:span-copy-code" label: "Copy artifact source" exit: #t))
        default-minor-modes: '()
        modeline-segments: '(polymode-inner)
        new-buffer-actions: '()
        enter-hooks: '()
        exit-hooks: '())))

;; 4. Polymode Host -> Inner Span Specs (AI-Native multi-modal buffers!)
(def DefaultPolymodes
  (list
    (.o id: 'poly-marlin-chat-mode
        host-mode: 'marlin-chat-mode
        inner-rules: (list
                       (.o span-kind: 'code-block inner-mode: 'prog-code-inner-mode lighter: "Code")
                       (.o span-kind: 'diff-hunk inner-mode: 'magit-diff-inner-mode lighter: "Diff·Aitia")
                       (.o span-kind: 'tool-gate inner-mode: 'marlin-gate-inner-mode lighter: "Gate")
                       (.o span-kind: 'artifact inner-mode: 'marlin-artifact-inner-mode lighter: "Artifact"))
        switch-span-hooks: '(update-modeline-polymode-badge))
    (.o id: 'poly-magit-status-mode
        host-mode: 'magit-status-mode
        inner-rules: (list
                       (.o span-kind: 'diff-hunk inner-mode: 'magit-diff-inner-mode lighter: "Hunk·Aitia")
                       (.o span-kind: 'conflict inner-mode: 'magit-diff-inner-mode lighter: "Conflict")
                       (.o span-kind: 'code-block inner-mode: 'prog-code-inner-mode lighter: "Code"))
        switch-span-hooks: '(update-modeline-polymode-badge))
    (.o id: 'poly-marlin-room-mode
        host-mode: 'marlin-room-mode
        inner-rules: (list
                       (.o span-kind: 'code-block inner-mode: 'prog-code-inner-mode lighter: "Code")
                       (.o span-kind: 'diff-hunk inner-mode: 'magit-diff-inner-mode lighter: "Diff·Aitia")
                       (.o span-kind: 'tool-gate inner-mode: 'marlin-gate-inner-mode lighter: "Gate"))
        switch-span-hooks: '(update-modeline-polymode-badge))))

;; 5. Buffer-to-Mode routing & Tab Bar "+" Buffer Picker catalog
(def DefaultBufferModeRules
  (list
    (.o prefix: "*chat:" major-mode: 'marlin-chat-mode creatable: #t create-label: "New LLM Chat (*chat:*)" create-action: "cmd:new-chat-buffer")
    (.o prefix: "*conversation*" major-mode: 'marlin-chat-mode creatable: #f create-label: "Conversation (*conversation*)" create-action: "buffer:*conversation*")
    (.o prefix: "*magit" major-mode: 'magit-status-mode creatable: #t create-label: "Magit Status (*magit*)" create-action: "cmd:magit-status")
    (.o prefix: "*Agent Room*" major-mode: 'marlin-room-mode creatable: #t create-label: "Agent Room (*Agent Room*)" create-action: "feature:chatroom")
    (.o prefix: "feature:chatroom" major-mode: 'marlin-room-mode creatable: #f create-label: "Agent Room" create-action: "feature:chatroom")
    (.o prefix: "*Workflow Canvas*" major-mode: 'marlin-workflow-mode creatable: #t create-label: "Workflow Canvas (*Workflow Canvas*)" create-action: "feature:webisland")
    (.o prefix: "feature:webisland" major-mode: 'marlin-workflow-mode creatable: #f create-label: "Workflow Canvas" create-action: "feature:webisland")
    (.o prefix: "*Dashboard*" major-mode: 'marlin-dashboard-mode creatable: #t create-label: "Dashboard (*Dashboard*)" create-action: "buffer:*Dashboard*")
    (.o prefix: "*Sessions*" major-mode: 'marlin-tabulated-mode creatable: #t create-label: "Sessions (*Sessions*)" create-action: "buffer:*Sessions*")
    (.o prefix: "*Ibuffer*" major-mode: 'marlin-tabulated-mode creatable: #t create-label: "Ibuffer (*Ibuffer* by Major Mode)" create-action: "cmd:ibuffer")
    (.o prefix: "*Targets*" major-mode: 'marlin-tabulated-mode creatable: #t create-label: "Targets (*Targets*)" create-action: "buffer:*Targets*")
    (.o prefix: "*Processes*" major-mode: 'marlin-tabulated-mode creatable: #t create-label: "Processes (*Processes*)" create-action: "buffer:*Processes*")
    (.o prefix: "*Models*" major-mode: 'marlin-tabulated-mode creatable: #t create-label: "Models (*Models*)" create-action: "buffer:*Models*")
    (.o prefix: "*Preview*" major-mode: 'marlin-preview-mode creatable: #f create-label: "Preview (*Preview*)" create-action: "buffer:*Preview*")
    (.o prefix: "*Help*" major-mode: 'marlin-help-mode creatable: #t create-label: "Help (*Help*)" create-action: "buffer:*Help*")
    (.o prefix: "*Messages*" major-mode: 'marlin-messages-mode creatable: #t create-label: "Messages (*Messages*)" create-action: "buffer:*Messages*")
    (.o prefix: "*Settings*" major-mode: 'marlin-settings-mode creatable: #t create-label: "Settings (*Settings*)" create-action: "buffer:*Settings*")
    (.o prefix: "*Packages*" major-mode: 'marlin-packages-mode creatable: #t create-label: "Packages (*Packages*)" create-action: "buffer:*Packages*")))
