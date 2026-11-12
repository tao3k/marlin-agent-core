;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme-owned terminal component composition and focus policy.
(.def Ui
  identity: 'ui
  core: (.o requires: (.o) scope-contract: UiScopeContract exports: UiScope)
  features: (.o))
