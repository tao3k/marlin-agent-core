;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(.def BriefToDraft
  identity: 'brief-to-draft
  core: (.o requires: (.o)
            scope-contract: BriefToDraftScopeContract
            exports: BriefToDraftScope)
  features: (.o))
