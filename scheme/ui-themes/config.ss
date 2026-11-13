;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/mop validate)
        "./objects.ss" "./types.ss")
(export UiThemesScope)

(def UiThemesScope
  (validate UiThemesScopeContract
            (.o identity: 'marlin/ui-themes
                selected: (marlin-ui-theme-for marlin-ui-default-profile)
                default-profile: marlin-ui-default-profile
                profiles: marlin-ui-theme-profiles
                select: marlin-ui-theme-for)))
