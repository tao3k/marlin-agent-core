;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


(import (only-in "./objects.ss" marlin-ui-theme-profiles))
(export marlin-ui-theme-for)

(def (marlin-ui-theme-for profile)
  (let (entry (assq profile marlin-ui-theme-profiles))
    (if entry (caddr entry) (error "unknown UI theme profile"))))
