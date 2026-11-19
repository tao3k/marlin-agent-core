;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./funcs.ss" "./types.ss")
(export UiCentaurTabsScope)

(def UiCentaurTabsScope
  (.new/checked UiCentaurTabsScopeContract
    identity: 'marlin/ui-centaur-tabs
    bar-for: marlin-ui-tabs-for-style))
