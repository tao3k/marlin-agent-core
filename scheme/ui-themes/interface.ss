;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Selected visual themes are UI data, independent of the terminal backend.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiThemesScope)
        (only-in "./types.ss" UiThemesScopeContract))
(export UiThemes)

(.def UiThemes
  identity: 'ui-themes
  core: (.o scope-contract: UiThemesScopeContract
            exports: UiThemesScope)
  features: (.o))
