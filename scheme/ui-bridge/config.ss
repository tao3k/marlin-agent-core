;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./funcs.ss"
        "./types.ss")
(export UiBridgeScope)

(def UiBridgeScope
  (.new/checked UiBridgeScopeContract
    identity: 'marlin/ui-bridge
    default-modules: DefaultBridgeModules
    list-namespaces: marlin-scheme-bridge-list-namespaces
    register-module: marlin-scheme-bridge-register!
    dispatch: marlin-scheme-bridge-dispatch))
