;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Host-neutral, bounded terminal layout policy.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiLayoutScope)
        (only-in "./types.ss" UiLayoutScopeContract))
(export UiLayout)

(.def UiLayout
  identity: 'ui-layout
  core: (.o scope-contract: UiLayoutScopeContract
            exports: UiLayoutScope))
