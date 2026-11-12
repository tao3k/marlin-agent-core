;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop validate) "./types.ss" "./objects.ss")
(export UiScope)
(def UiScope (validate UiScopeContract UiOperations))
