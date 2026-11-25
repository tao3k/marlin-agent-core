;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Modern Helm Workbench Package:
;;; Projects a live `HelmSession.` into a declarative `*Helm*` TUI buffer and
;;; `SPC f h` keybinding inside a standard checked Marlin `UiScope`.

(import (only-in :clan/poo/object .get .o)
        (only-in "../ui-helm/funcs.ss" helm-project-ui-elements)
        (only-in "../ui-meow/interface.ss" define-key-bindings meow-normal)
        (only-in "./package.ss" UiPackage. make-ui-package))

(export HelmWorkbenchKeymap.
        marlin-helm-workbench-buffers
        make-marlin-helm-workbench-package)

;;; Disjoint from `UiMeowScope` (`SPC s` opens live interactive Helm, `SPC x`,
;;; `SPC b b`, `SPC r`, `SPC t t`, `SPC h`) and `TaskWorkbenchKeymap.` (`SPC o`).
(define-key-bindings HelmWorkbenchKeymap.
  (meow-normal
    ((SPC f h) buffer-helm-snapshot "Focus Helm results buffer")))

(def (marlin-helm-workbench-buffers helm-session)
  (list
    (list 'helm-snapshot
          "Helm multi-source search snapshot"
          (list "*Helm*" (helm-project-ui-elements helm-session))
          #f)))

(def (make-marlin-helm-workbench-package helm-session)
  (make-ui-package
    (.o (:: @ UiPackage.)
        buffers: (marlin-helm-workbench-buffers helm-session)
        commands: '()
        keymap: (.get HelmWorkbenchKeymap. bindings))))
