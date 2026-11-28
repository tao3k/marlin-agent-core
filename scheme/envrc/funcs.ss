;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :clan/poo/type validate)
        (only-in "./types.ss" EnvrcRequest))

(export plan-envrc)

(def (plan-envrc request)
  (def observation (validate EnvrcRequest request))
  (if (.get observation project-envrc?)
    (case (.get observation intent)
      ((activate) 'export)
      ((allow) 'allow-and-export)
      ((reload) 'reload-and-export)
      ((deny) 'deny))
    'absent))
