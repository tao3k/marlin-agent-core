;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Policy operations exposed through Marlin's Yuanxu selection.

(.def Policy
  identity: 'policy
  core: (.o scope-contract: PolicyScopeContract
            exports: PolicyScope)
  features: (.o))
