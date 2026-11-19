;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme-owned component composition and focus policy for native hosts.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiScope)
        (only-in "./types.ss" UiScopeContract))
(export Ui)

(.def Ui
  identity: 'ui
  core: (.o scope-contract: UiScopeContract
            exports: UiScope))
