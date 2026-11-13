;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(.def WorkingCopy
  identity: 'working-copy
  core: (.o requires: (.o)
            scope-contract: WorkingCopyScopeContract
            exports: WorkingCopyScope
            projections: (list Build))
  features: (.o))
