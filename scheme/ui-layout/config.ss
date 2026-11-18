;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/type validate)
        "./objects.ss" "./types.ss")
(export UiLayoutScope marlin-ui-layout-with-tab-bar)

(def UiLayoutScope
  (validate UiLayoutScopeContract
            (.o identity: 'marlin/ui-layout policy-for: marlin-ui-layout-policy-for)))

(def (marlin-ui-layout-with-tab-bar tab-bar)
  (validate UiLayoutScopeContract
    (.o (:: @ UiLayoutScope)
        policy-for:
        (lambda (modeline transcript buffers)
          (marlin-ui-layout-policy-for modeline transcript buffers tab-bar)))))
