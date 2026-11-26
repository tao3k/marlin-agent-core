;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative Doom Emacs-inspired Workspace (persp-mode) & Popup Window Rule
;;; (set-popup-rule!) module for Marlin.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiWorkspacesScope)
        (only-in "./types.ss" UiWorkspacesScopeContract))
(export UiWorkspaces)

(.def UiWorkspaces
  identity: 'ui-workspaces
  core: (.o scope-contract: UiWorkspacesScopeContract
            exports: UiWorkspacesScope))
