;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Or Record Symbol Tuple Type.)
        (only-in :tao3k/yuanxu/prelude YuanxuLibraryModule)
        (only-in "../scheme-package-hooks/types.ss"
                 UiPackageDirectory UiPackageDemand UiPackageCommandName)
        (only-in "../ui-components/bounds.ss" marlin-ui-bounded-list?)
        (only-in "../ui-components/types.ss" UiCommandEntries)
        (only-in "../ui-meow/types.ss" KeyBindings))
(export UiPackageDeclaration UiPackageEventInput)

(def UiPackageBufferHeaders
  (.cc Type. '.element?
       (lambda (entries)
         (and (marlin-ui-bounded-list? entries 8)
              (andmap (lambda (entry)
                        (and (marlin-ui-bounded-list? entry 3) (= (length entry) 3)
                             (symbol? (car entry)) (string? (cadr entry))
                             (<= (string-length (cadr entry)) 256)
                             (or (not (caddr entry)) (string? (caddr entry)))))
                      entries)))))

(def UiPackageDeclaration
  (Record module: [YuanxuLibraryModule] export: [Symbol] buffers: [UiPackageBufferHeaders]
          commands: [UiCommandEntries] keymap: [KeyBindings]))

(def UiPackageEventInput
  (Or (Tuple (Exactly 'catalog) UiPackageDirectory YuanxuLibraryModule Symbol)
      (Tuple UiPackageDemand UiPackageCommandName)))
