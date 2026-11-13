;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Reusable component vocabulary and declarative command catalog.
(.def UiComponents
  identity: 'ui-components
  core: (.o requires: (.o UiHydra: (.o module-id: 'ui-hydra)
                          UiElements: (.o module-id: 'ui-elements))
            scope-contract: UiComponentsScopeContract
            exports: (make-ui-components-scope UiHydra))
  features: (.o))
