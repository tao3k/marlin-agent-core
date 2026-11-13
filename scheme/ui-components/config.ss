;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :clan/poo/mop element? validate)
        (only-in "../ui-hydra/types.ss" UiHydraScopeContract)
        "./types.ss" "./objects.ss")
(export make-ui-components-scope)
(def (make-ui-components-scope hydra)
  (unless (element? UiHydraScopeContract hydra) (error "invalid selected hydra scope"))
  (validate UiComponentsScopeContract
            (make-ui-components-operations (.get hydra command-entries) (.get hydra entries))))
