;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative AI-Native Magit (jj + Git + Human-AI Code Audit + Aitia)
;;; policy and authentic Doom Emacs keybinding module.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiMagitScope)
        (only-in "./types.ss" UiMagitScopeContract))
(export UiMagit)

(.def UiMagit
  identity: 'ui-magit
  core: (.o scope-contract: UiMagitScopeContract
            exports: UiMagitScope))
