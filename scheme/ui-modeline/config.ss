;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/mop validate)
        "./objects.ss" "./types.ss")
(export UiModelineScope)

(def UiModelineScope
  (validate UiModelineScopeContract
            (.o identity: 'marlin/ui-modeline nodes: marlin-ui-modeline-nodes)))
