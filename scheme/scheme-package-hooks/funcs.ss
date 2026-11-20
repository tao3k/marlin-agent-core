;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .call .get)
        (only-in :tao3k/yuanxu/prelude yuanxu-module-export))
(export load-configured-publication run-ui-package-hooks)

(def (load-configured-publication request publication)
  (or publication
      (parameterize ((current-directory (path-expand (.get request directory))))
        (yuanxu-module-export (.get request module) (.get request export)))))

;;; Post-load hooks see an available publication and can compose its value
;;; before the native UI owner admits it into the current session.
(def (run-ui-package-hooks hooks event request)
  (let (publication (.call hooks .run event request #f))
    (unless publication (error "no Scheme package hook handled the UI demand" event))
    (.call hooks .run 'ui-package-loaded request publication)))
