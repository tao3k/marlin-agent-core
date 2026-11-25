;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Search composition belongs to the real Gerbil Scheme module system and
;;; projects across Marlin's rooted C ABI into ASP's Rust search engine.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" SearchEngineScope)
        (only-in "./types.ss" SearchEngineScopeContract))
(export SearchEngine)

(.def SearchEngine
  identity: 'search-engine
  core: (.o scope-contract: SearchEngineScopeContract
            exports: SearchEngineScope))
