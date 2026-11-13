;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Selected visual themes are UI data, independent of the terminal backend.
(.def UiThemes
  identity: 'ui-themes
  core: (.o requires: (.o) scope-contract: UiThemesScopeContract
            exports: UiThemesScope)
  features: (.o))
