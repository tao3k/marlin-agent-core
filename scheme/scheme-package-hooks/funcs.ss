;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .call .get)
        (only-in :clan/poo/type validate)
        (only-in :tao3k/yuanxu/module-system/module-hooks make-module-hooks)
        (only-in :tao3k/yuanxu/prelude yuanxu-module-declaration)
        (only-in "../ui/types.ss" UiScopeContract)
        (only-in "./types.ss" SchemePackageHooksScopeContract))
(export load-configured-publication make-ui-package-hooks run-ui-package-hooks)

(def (make-ui-package-hooks declarations)
  (validate SchemePackageHooksScopeContract (make-module-hooks declarations)))

(def (load-configured-publication request publication)
  (or publication
      (yuanxu-module-declaration (.get request module) (.get request export) UiScopeContract
        directory: (.get request directory))))

;;; Post-load hooks see an available publication and can compose its value
;;; before the native UI owner admits it into the current session.
(def (run-ui-package-hooks hooks request)
  (let* ((demand (.get request trigger))
         (publication (.call hooks .run demand request #f)))
    (unless publication (error "no Scheme package hook handled the UI demand" demand))
    (or (.call hooks .run 'ui-package-loaded request publication)
        (error "Scheme package hook discarded the loaded UI publication" demand))))
