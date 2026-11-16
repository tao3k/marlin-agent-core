;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The terminal programme is an ordinary selected Module and Build role.
(.def UiTui
  identity: 'ui-tui
  core: (.o scope-contract: UiTuiScopeContract
            exports: UiTuiScope
            projections: (list Build))
  features: (.o))
