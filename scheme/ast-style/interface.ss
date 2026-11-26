;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Frontier 1 Machine-Native Relational Coding Style Engine publication.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" AstStyleScope)
        (only-in "./types.ss" AstStyleScopeContract))
(export AstStyle)

(.def AstStyle
  identity: 'ast-style
  core: (.o scope-contract: AstStyleScopeContract
            exports: AstStyleScope))
