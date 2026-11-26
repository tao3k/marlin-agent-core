;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Unified `:gerbil-ascent` + `:gerbil-parser` + POO Search DSL module publication.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" SearchDslScope)
        (only-in "./types.ss" SearchDslScopeContract))
(export SearchDsl)

(.def SearchDsl
  identity: 'search-dsl
  core: (.o scope-contract: SearchDslScopeContract
            exports: SearchDslScope))
