;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Type.)
        (only-in :clan/poo/type Exactly Record))
(export UiElementsScopeContract)
(def UiElementValues (.cc Type. '.element? list?))
(def UiElementVector (.cc Type. '.element? vector?))
(def UiElementIdentity (.cc Type. '.element? symbol?))
(def UiElementFlag (.cc Type. '.element? boolean?))
(def UiElementsScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-elements)]
               text: [(Fun UiElementVector <- UiElementValues)]
               rows: [(Fun UiElementVector <- UiElementValues)]
               columns: [(Fun UiElementVector <- UiElementValues)]
               template: [(Fun UiElementVector <- UiElementIdentity UiElementValues)]
               choice: [(Fun UiElementVector <- UiElementValues UiElementVector)]
               region: [(Fun UiElementVector <- UiElementIdentity UiElementFlag UiElementVector)])
       identity: 'marlin/ui-elements))
