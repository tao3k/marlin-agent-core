;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Enum Exactly Fun Or OrFalse Record Symbol Tuple Type.)
        (only-in "../ui/types.ss" UiScopeContract))
(export UiPackageDemandInput SchemePackageHooksScopeContract)

(def UiPackageDirectory
  (.cc Type. '.element?
       (lambda (value) (and (string? value) (<= 1 (string-length value) 4096)))))
(def UiPackageDemand (Enum ui-command-catalog ui-keymap-miss))
(def UiPackageHookEvent (Or UiPackageDemand (Exactly 'ui-package-loaded)))
(def UiPackageDemandInput (Tuple UiPackageDirectory Symbol Symbol UiPackageDemand))
(def UiPackageRequest
  (Record directory: [UiPackageDirectory] module: [Symbol] export: [Symbol]
          trigger: [UiPackageDemand]))
(def UiPackageHookResult (OrFalse UiScopeContract))

(def SchemePackageHooksScopeContract
  (.cc (Record .run: [(Fun UiPackageHookResult <- UiPackageHookEvent
                           UiPackageRequest UiPackageHookResult)])
       identity: 'marlin/scheme-package-hooks))
