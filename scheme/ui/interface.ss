;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme-owned component composition and focus policy for native hosts.
(.def Ui
  identity: 'ui
  core: (.o requires: (.o UiComponents: (.o module-id: 'ui-components))
            scope-contract: UiScopeContract
            exports: (make-ui-scope UiComponents))
  features: (.o))
