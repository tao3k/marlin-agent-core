;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Policy operations exposed through Marlin's Yuanxu selection.

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" PolicyScope)
        (only-in "./types.ss" PolicyScopeContract))
(export Policy)

(.def Policy
  identity: 'policy
  core: (.o scope-contract: PolicyScopeContract
            exports: PolicyScope))
