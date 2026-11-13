;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Host-neutral mode line slots. The terminal owns cells and colors.
(.def UiModeline
  identity: 'ui-modeline
  core: (.o requires: (.o) scope-contract: UiModelineScopeContract
            exports: UiModelineScope)
  features: (.o))
