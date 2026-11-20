;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The qualification programme has its own link product.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" BenchScope)
        (only-in "./types.ss" BenchScopeContract))

(export Bench)

(.def Bench
  identity: 'bench
  core: (.o scope-contract: BenchScopeContract
            exports: BenchScope))
