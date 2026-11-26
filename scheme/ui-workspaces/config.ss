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
    project-state: marlin-ws-project-state))
