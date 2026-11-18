;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in "./config.ss" EnvrcScope))

(export plan-project-envrc)

(def (plan-project-envrc request)
  (unless (and (vector? request) (= 2 (vector-length request)))
    (error "expected an ENVRC intent and project observation"))
  ;; The package Builder selects ENVRC before including this export. Using the
  ;; checked scope keeps the linked terminal entry independent of Lab startup.
  ((.get EnvrcScope plan)
   (.o intent: (vector-ref request 0) project-envrc?: (vector-ref request 1))))
