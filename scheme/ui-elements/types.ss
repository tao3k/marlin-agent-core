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
(def UiScrollStep (.cc Type. '.element? (lambda (value) (and (exact-integer? value) (<= 1 value 32)))))
(def UiBarDivisor (.cc Type. '.element? (lambda (value) (and (exact-integer? value) (<= 1 value 16)))))
(def UiScrollTarget (.cc Type. '.element?
  (lambda (value) (or (not value)
    (and (vector? value) (= (vector-length value) 2) (symbol? (vector-ref value 0))
         (exact-integer? (vector-ref value 1)) (<= 1 (vector-ref value 1) 32))))))
(def UiElementsScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-elements)]
               text: [(Fun UiElementVector <- UiElementValues)]
               rows: [(Fun UiElementVector <- UiElementValues)]
               columns: [(Fun UiElementVector <- UiElementValues)]
               template: [(Fun UiElementVector <- UiElementIdentity UiElementValues)]
               prompt: [(Fun UiElementVector <- UiElementIdentity UiElementVector)]
               bar: [(Fun UiElementVector <- UiElementIdentity UiBarDivisor UiElementValues)]
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
               list: [(Fun UiElementVector <- UiElementIdentity UiElementVector UiElementVector)]
               sized: [(Fun UiElementVector <- UiElementVector UiElementVector)]
               region: [(Fun UiElementVector <- UiElementIdentity UiScrollTarget UiElementVector)]
               scroll-target: [(Fun UiElementVector <- UiElementIdentity UiScrollStep)])
       identity: 'marlin/ui-elements))
