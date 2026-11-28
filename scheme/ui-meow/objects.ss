;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./funcs.ss" "./types.ss")
(export make-marlin-ui-keymap)

(def (make-marlin-ui-keymap bindings-value)
  (.new/checked UiMeowScopeContract
    identity: 'marlin/ui-meow
    bindings: bindings-value
    project: marlin-ui-project-keymap
    help: marlin-ui-format-keymap))
