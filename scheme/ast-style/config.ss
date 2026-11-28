;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validated publication for Marlin's Frontier 1 Machine-Native Relational
;;; Coding Style Engine (`scheme/ast-style`).
(import (only-in :clan/poo/mop .new/checked)
        "./types.ss"
        "./objects.ss"
        "./funcs.ss")
(export AstStyleScope)

(def AstStyleScope
  (.new/checked AstStyleScopeContract
    identity: 'marlin/ast-style
    registry: CodingStyleRegistryValue
    evaluate-edb: (lambda (edb) (evaluate-coding-style-over-edb edb CodingStyleRegistryValue))
    project-contract: (lambda (lang target types)
                        (project-symbolic-style-contract lang target types CodingStyleRegistryValue))
    cost-tuple: compute-style-cost-tuple
    cost-less?: style-cost<?))
