;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        "./funcs.ss")
(export UiElementOperations)

(def UiElementOperations
  (.o identity: 'marlin/ui-elements
      text: ui-text rows: ui-rows columns: ui-columns
      region: ui-region scroll-target: ui-scroll-target
      template: ui-template prompt: ui-prompt bar: ui-bar choice: ui-choice
      size-cells: ui-size-cells size-flex: ui-size-flex size-binding: ui-size-binding
      size-bounded: ui-size-bounded size-choice: ui-size-choice sized: ui-sized-with
      size-min: ui-size-min size-max: ui-size-max
      size-add: ui-size-add size-subtract: ui-size-subtract list: ui-list))
