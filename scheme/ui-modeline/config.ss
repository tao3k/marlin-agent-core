;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./funcs.ss" "./types.ss")
(export UiModelineScope)

(def UiModelineScope
  (.new/checked UiModelineScopeContract
    identity: 'marlin/ui-modeline nodes: marlin-ui-modeline-nodes
    nodes-for: marlin-ui-modeline-nodes-for bar-for: marlin-ui-modeline-bar-for
    preset-spec: marlin-ui-modeline-preset-spec
    visible-segments: marlin-ui-modeline-visible-segments
    shrink-path: marlin-ui-modeline-shrink-path))
