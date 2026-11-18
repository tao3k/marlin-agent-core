;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Reusable component vocabulary and declarative command catalog.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiComponentsScope)
        (only-in "./types.ss" UiComponentsScopeContract))
(export UiComponents)

(.def UiComponents
  identity: 'ui-components
  core: (.o scope-contract: UiComponentsScopeContract
            exports: UiComponentsScope)
  features: (.o))
