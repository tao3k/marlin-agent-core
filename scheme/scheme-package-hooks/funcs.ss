;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :tao3k/yuanxu/prelude yuanxu-module-export))
(export load-configured-publication)

(def (load-configured-publication request publication)
  (or publication
      (parameterize ((current-directory (path-expand (.get request directory))))
        (yuanxu-module-export (.get request module) (.get request export)))))
