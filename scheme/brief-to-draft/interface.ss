;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" BriefToDraftScope)
        (only-in "./types.ss" BriefToDraftScopeContract))
(export BriefToDraft)

(.def BriefToDraft
  identity: 'brief-to-draft
  core: (.o scope-contract: BriefToDraftScopeContract
            exports: BriefToDraftScope))
