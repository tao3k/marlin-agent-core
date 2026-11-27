;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative Emacs Major Mode, Minor Mode, Hooks, and AI-Native Polymode module.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiModesScope)
        (only-in "./types.ss" UiModesScopeContract))
(export UiModes)

(.def UiModes
  identity: 'ui-modes
  core: (.o scope-contract: UiModesScopeContract
            exports: UiModesScope))
