;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        (only-in "./objects.ss" DefaultPopupRules)
        "./funcs.ss"
        "./types.ss")
(export UiWorkspacesScope)

(def UiWorkspacesScope
  (.new/checked UiWorkspacesScopeContract
    identity: 'marlin/ui-workspaces
    default-popup-rules: DefaultPopupRules
    make-state: marlin-ws-make-state
    make-popup-rule: marlin-ws-make-popup-rule
    add-popup-rule: marlin-ws-add-popup-rule
    match-popup-rule: marlin-ws-match-popup-rule
    create-workspace: marlin-ws-create
    switch-workspace: marlin-ws-switch
    kill-workspace: marlin-ws-kill
    open-buffer: marlin-ws-open-buffer
    dismiss-popup: marlin-ws-dismiss-popup
    toggle-popup: marlin-ws-toggle-popup
    raise-popup: marlin-ws-raise-popup
    workspace-buffers: marlin-ws-buffers
    all-workspace-buffers: marlin-ws-all-buffers
    isolate-buffer-candidates: marlin-ws-isolate-candidates
    window-list: marlin-ws-window-list
    window-count: marlin-ws-window-count
    focused-window: marlin-ws-focused-window
    window-number: marlin-ws-window-number
    focused-window-number: marlin-ws-focused-window-number
    split-window: marlin-ws-split-window
    select-window-number: marlin-ws-select-window-number
    other-window: marlin-ws-other-window
    delete-window: marlin-ws-delete-window
    delete-other-windows: marlin-ws-delete-other-windows
    swap-windows: marlin-ws-swap-windows
    resize-window: marlin-ws-resize-window
    balance-windows: marlin-ws-balance-windows
    toggle-dedicated: marlin-ws-toggle-dedicated
    winner-undo: marlin-ws-winner-undo
    winner-redo: marlin-ws-winner-redo
    modeline-window-indicator: marlin-ws-modeline-window-indicator
    project-window-tree: marlin-ws-project-window-tree
    project-state: marlin-ws-project-state
    project-popup-rules: marlin-ws-project-popup-rules))
