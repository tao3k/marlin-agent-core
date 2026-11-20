;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


(import (only-in :clan/poo/object .get))
(export project-tooling)

(def (project-tool declaration)
  (vector (.get declaration name)
          (.get declaration description)
          (.get declaration capability)
          (.get declaration governance)
          (.get declaration argument)
          (.get declaration timeout-seconds)
          (.get declaration output-bytes)))

(def (project-tooling catalog)
  (list->vector (map project-tool (.get catalog declarations))))
