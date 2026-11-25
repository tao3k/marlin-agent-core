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
;;; The inherited slot computes one ordered catalog when the selected module reads it.
(def marlin-ui-command-catalog
  (seal-object! (.mix WorkspaceCommands. JournalCommands. UiCommandCatalog.)))
