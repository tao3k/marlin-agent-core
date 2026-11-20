;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


(import (only-in "./objects.ss" doom-transcript-policy pi-transcript-policy))
(export marlin-ui-transcript-policy-for)

(def (marlin-ui-transcript-policy-for profile)
  (case profile
    ((doom-one) doom-transcript-policy)
    ((pi) pi-transcript-policy)
    (else (error "unknown UI transcript profile"))))
