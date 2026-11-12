;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop element? validate)
        (only-in "../ui-components/types.ss" UiComponentsScopeContract)
        "./types.ss" "./objects.ss")
(export make-ui-scope)
(def (make-ui-scope components)
  (unless (element? UiComponentsScopeContract components)
    (error "invalid selected UI components scope"))
  (validate UiScopeContract (make-ui-operations components)))
