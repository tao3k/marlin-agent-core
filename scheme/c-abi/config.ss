;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop validate)
        "./types.ss" "./objects.ss")
(export make-c-abi-scope)
(def (make-c-abi-scope ui context deck)
  (validate CAbiScopeContract (make-c-abi-operations ui context deck)))
