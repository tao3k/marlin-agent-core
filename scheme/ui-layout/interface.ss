;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Host-neutral, bounded terminal layout policy.
(.def UiLayout
  identity: 'ui-layout
  core: (.o requires: (.o) scope-contract: UiLayoutScopeContract
            exports: UiLayoutScope)
  features: (.o))
