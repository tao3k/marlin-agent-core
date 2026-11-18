;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validate the Context publication.

(import (only-in :clan/poo/type validate)
        "./types.ss"
        "./objects.ss")

(export MarlinContextSystemScope)

(def MarlinContextSystemScope
  (validate MarlinContextSystemContract MarlinContextSystemOperations))
