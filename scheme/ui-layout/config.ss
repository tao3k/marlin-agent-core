;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in :clan/poo/mop .new/checked)
        (only-in :clan/poo/type validate)
        (only-in "../ui-centaur-tabs/config.ss" UiCentaurTabsScope)
        "./funcs.ss" "./types.ss")
(export UiLayoutScope marlin-ui-layout-with-tab-bar)

(def UiLayoutScope
  (.new/checked UiLayoutScopeContract
    identity: 'marlin/ui-layout
    policy-for: marlin-ui-layout-policy-for
    tab-bar-for: (.get UiCentaurTabsScope bar-for)))

(def (marlin-ui-layout-with-tab-bar tab-bar)
  (validate UiLayoutScopeContract
    (.o (:: @ UiLayoutScope)
        tab-bar-for: tab-bar)))
