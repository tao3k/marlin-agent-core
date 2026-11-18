;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def)
        (only-in "./objects.ss" select-host-working-copy-provider))
(export GitOpsHostOperations)

(.def GitOpsHostOperations
  select-host-provider: select-host-working-copy-provider)
