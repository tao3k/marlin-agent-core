;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


;;; Command names, capability defaults and ordering are declared once.
(import (only-in :clan/poo/object .def .mix seal-object!)
        (only-in "./buffers.ss" marlin-ui-buffer-command-entries marlin-ui-buffer-alias-command-entries))
(export marlin-ui-command-catalog)

(.def UiCommandCatalog.
  entries: (append (marlin-ui-buffer-alias-command-entries)
                   '(("models" "List configured models" none models #f)
                     ("model" "Select a model alias" required model #f))))
;;; Journal recovery remains a storage operation, not a Context source.
(.def JournalCommands.
  (entries => append '(("sessions" "Browse journal IDs; next for another page" optional sessions #f)
                       ("session" "Recover a quoted journal ID" required session #f)
                       ("inspect" "Read a quoted journal ID" required inspect #f))))
(.def WorkspaceCommands.
  (entries => append '(("status" "Show workspace status" none workspace "status")
                       ("diff" "Show unstaged or staged changes" optional workspace "diff")
                       ("worktree-switch" "Switch to a local branch with Worktrunk" required workspace "worktree-switch")
                       ("context-budget" "Show request budget" none context-budget #f)
                       ("runtime" "Show runtime policy" none runtime #f)
                       ("list-processes" "List managed Runtime processes"
                        none select-buffer "processes")
                       ("kill-process" "Stop a managed process by generation"
                        required kill-process #f)
                       ("envrc-allow" "Allow the current worktree .envrc" none envrc "allow")
                       ("envrc-reload" "Reload the current worktree environment" none envrc "reload")
                       ("envrc-deny" "Deny the current worktree .envrc" none envrc "deny")
                       ("envrc-cancel" "Cancel the current environment load" none envrc "cancel")
                       ("envrc-status" "Show the current worktree environment status" none envrc "status"))
                     '(("switch-themes" "Theme: " none open-menu "theme-")
                       ("centaur-tabs-themes-switch" "Tab style: " none open-menu "tabs-")
                       ("switch-buffer" "Switch to: " none open-menu "buffer-")
                       ("helm" "Open interactive Helm search" optional helm #f))
                     (marlin-ui-buffer-command-entries)
                     '(("toggle-thinking" "Expand or fold thinking (N from newest)"
                        optional toggle-thinking #f)
                       ("clear" "Clear displayed transcript" none clear #f))))
(.def TaskTabCommands.
  (entries => append '(("task-tab-index" "Focus the pinned Tasks Index tab" none scheme-action "task-tab-index")
                       ("task-tab-next" "Focus the next active TaskActor tab" none scheme-action "task-tab-next")
                       ("task-tab-prev" "Focus the previous active TaskActor tab" none scheme-action "task-tab-prev")
                       ("task-tab-focus" "Focus an active TaskActor tab by ID" required scheme-action "task-tab-focus")
                       ("task-tab-peek" "Open a read-only peek tab for an archived task ID" required scheme-action "task-tab-peek")
                       ("task-tab-unpeek" "Close a read-only peek tab by task ID" required scheme-action "task-tab-unpeek"))))
;;; Doom Emacs canonical window, winum, winner-mode, popup & persp-mode commands.
(.def WindowWorkspaceCommands.
  (entries => append '(("split-window-right" "Split focused window vertically left | right (SPC w v)" none scheme-action "window-split-right")
                       ("split-window-below" "Split focused window horizontally top / bottom (SPC w s)" none scheme-action "window-split-below")
                       ("other-window" "Cycle focus to next window in winum order (SPC w w)" none scheme-action "window-other")
                       ("delete-window" "Close focused window or non-main workspace (SPC w d)" none scheme-action "window-delete")
                       ("delete-other-windows" "Collapse split panes to focused window (SPC w o)" none scheme-action "window-only")
                       ("swap-window-panes" "Swap focused window with next window (SPC w x)" none scheme-action "window-swap")
                       ("balance-windows" "Reset all split ratios to 50/50 (SPC w =)" none scheme-action "window-balance")
                       ("toggle-window-dedicated" "Pin or unpin focused window to its buffer (SPC w t)" none scheme-action "window-toggle-dedicated")
                       ("winner-undo" "Undo previous window layout in active workspace (SPC w u)" none scheme-action "winner-undo")
                       ("winner-redo" "Redo undone window layout in active workspace (SPC w U)" none scheme-action "winner-redo")
                       ("popup-toggle" "Toggle active workspace's Doom popup drawer (SPC ~)" none scheme-action "popup-toggle")
                       ("popup-raise" "Promote active popup drawer into a main window (SPC w r)" none scheme-action "popup-raise")
                       ("workspace-new" "Create a new isolated persp-mode workspace (SPC TAB n)" none scheme-action "workspace-new")
                       ("workspace-kill" "Kill active workspace if others remain (SPC TAB d)" none scheme-action "workspace-kill"))))
;;; The inherited slot computes one ordered catalog when the selected module reads it.
(def marlin-ui-command-catalog
  (seal-object! (.mix WindowWorkspaceCommands. TaskTabCommands. WorkspaceCommands. JournalCommands. UiCommandCatalog.)))
