;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/type validate)
        (only-in "./objects.ss" BenchOperations)
        (only-in "./types.ss" BenchScopeContract))

(export BenchScope)

(def BenchScope (validate BenchScopeContract BenchOperations))
