;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Type.)
        (only-in :clan/poo/type Exactly Record))
(export UiElementsScopeContract)
(def UiElementValues (.cc Type. '.element? list?))
(def UiElementVector (.cc Type. '.element? vector?))
(def UiElementIdentity (.cc Type. '.element? symbol?))
(def UiSizeCount (.cc Type. '.element? (lambda (value) (and (exact-integer? value) (<= 0 value 65535)))))
(def UiScrollTarget (.cc Type. '.element? (lambda (value) (or (not value) (symbol? value)))))
(def UiElementsScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-elements)]
               text: [(Fun UiElementVector <- UiElementValues)]
               rows: [(Fun UiElementVector <- UiElementValues)]
               columns: [(Fun UiElementVector <- UiElementValues)]
               template: [(Fun UiElementVector <- UiElementIdentity UiElementValues)]
               choice: [(Fun UiElementVector <- UiElementValues UiElementVector)]
               size-cells: [(Fun UiElementVector <- UiSizeCount)]
               size-flex: [(Fun UiElementVector <- UiSizeCount)]
               size-binding: [(Fun UiElementVector <- UiElementIdentity)]
               size-bounded: [(Fun UiElementVector <- UiElementIdentity UiElementIdentity)]
               size-min: [(Fun UiElementVector <- UiElementVector UiElementVector)]
               size-max: [(Fun UiElementVector <- UiElementVector UiElementVector)]
               size-add: [(Fun UiElementVector <- UiElementVector UiElementVector)]
               size-subtract: [(Fun UiElementVector <- UiElementVector UiElementVector)]
               size-choice: [(Fun UiElementVector <- UiElementValues UiElementVector)]
               sized: [(Fun UiElementVector <- UiElementVector UiElementVector)]
               region: [(Fun UiElementVector <- UiElementIdentity UiScrollTarget UiElementVector)])
       identity: 'marlin/ui-elements))
