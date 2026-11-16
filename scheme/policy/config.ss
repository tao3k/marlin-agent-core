;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Select the contract-checked Policy scope.

(import (only-in :clan/poo/mop validate)
        "./objects.ss"
        "./types.ss")

(export PolicyScope)

(def PolicyScope (validate PolicyScopeContract PolicyOperations))
