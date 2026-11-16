;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validate the Context publication.

(import (only-in :clan/poo/mop validate)
        "./types.ss"
        "./objects.ss")

(export MarlinContextScope)

(def MarlinContextScope
  (validate MarlinContextModuleContract MarlinContextOperations))
