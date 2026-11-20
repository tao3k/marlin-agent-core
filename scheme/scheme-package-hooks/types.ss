;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Enum Exactly Fun Or OrFalse Record Symbol Type.)
        (only-in :tao3k/yuanxu/prelude YuanxuLibraryModule)
        (only-in "../ui-components/bounds.ss" marlin-ui-command-name?)
        (only-in "../ui/types.ss" UiScopeContract))
(export UiPackageDirectory UiPackageDemand UiPackageCommandName SchemePackageHooksScopeContract)

(def UiPackageDirectory
  (.cc Type. '.element?
       (lambda (value) (and (string? value) (<= 1 (string-length value) 4096)))))
(def UiPackageDemand (Enum ui-command-execution ui-key-command))
(def UiPackageCommandName (.cc Type. '.element? marlin-ui-command-name?))
(def UiPackageHookEvent (Or UiPackageDemand (Exactly 'ui-package-loaded)))
(def UiPackageRequest
  (Record directory: [UiPackageDirectory] module: [YuanxuLibraryModule] export: [Symbol]
          trigger: [UiPackageDemand] command: [UiPackageCommandName]))
(def UiPackageHookResult (OrFalse UiScopeContract))

(def SchemePackageHooksScopeContract
  (.cc (Record .run: [(Fun UiPackageHookResult <- UiPackageHookEvent
                           UiPackageRequest UiPackageHookResult)])
       identity: 'marlin/scheme-package-hooks))
