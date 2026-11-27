;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./funcs.ss"
        "./types.ss")
(export UiMygoScope)

(def UiMygoScope
  (.new/checked UiMygoScopeContract
    identity: 'marlin/ui-mygo
    profile-for: marlin-mygo-profile-for
    layout-scope-for: marlin-mygo-layout-scope-for
    slot-declarations-for: marlin-mygo-slot-declarations-for
    modules-for: marlin-mygo-modules-for
    admit-plugin-slot?: marlin-mygo-admit-plugin-slot?
    project-profile-overlay: marlin-mygo-project-overlay
    project-surface-manifest: marlin-mygo-project-surface-manifest
    make-mygo-ui-scope: marlin-mygo-make-ui-scope))
