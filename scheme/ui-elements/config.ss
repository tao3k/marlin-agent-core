;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/mop validate)
        "./objects.ss" "./types.ss")
(export UiElementsScope)
(def UiElementsScope
  (validate UiElementsScopeContract
    (.o identity: 'marlin/ui-elements text: ui-text rows: ui-rows columns: ui-columns
        region: ui-region template: ui-template choice: ui-choice)))
