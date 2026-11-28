;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Type contracts for Marlin's Frontier 1 Machine-Native Relational Coding Style
;;; Engine (`scheme/ast-style`).
(import (only-in :clan/poo/object .cc object?)
        (only-in :clan/poo/type Exactly Fun Record String Type.))
(export AstStyleAny
        AstStyleRegistryObject
        AstStyleScopeContract)

(def AstStyleAny (.cc Type. '.element? (lambda (_) #t)))
(def AstStyleRegistryObject (.cc Type. '.element? object?))

(def AstStyleScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ast-style)]
               registry: [AstStyleRegistryObject]
               evaluate-edb: [(Fun AstStyleAny <- AstStyleAny)]
               project-contract: [(Fun String <- String String AstStyleAny)]
               cost-tuple: [(Fun AstStyleAny <- AstStyleAny)]
               cost-less?: [(Fun AstStyleAny <- AstStyleAny AstStyleAny)])
       identity: 'marlin/ast-style))
