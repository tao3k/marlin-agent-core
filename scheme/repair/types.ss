;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Yuanxu POO type contracts for Marlin's `repair` module and its opt-in
;;; `+ast-patch` feature (`scheme/repair`).
(import (only-in :clan/poo/object .cc object?)
        (only-in :clan/poo/type Exactly Fun Object Record String Type.))
(export RepairAny
        RepairBoolean
        RepairVector
        RepairPolicyObject
        RepairScopeContract
        RepairAstPatchOpsContract
        RepairAstPatchContract)

(def RepairAny (.cc Type. '.element? (lambda (_) #t)))
(def RepairBoolean (.cc Type. '.element? boolean?))
(def RepairVector (.cc Type. '.element? vector?))
(def RepairPolicyObject (.cc Type. '.element? object?))

(def RepairScopeContract
  (.cc (Record identity: [(Exactly 'marlin/repair)]
               policy: [RepairPolicyObject]
               read-and-repair: [(Fun RepairVector <- String RepairAny)]
               repair-ast: [(Fun RepairVector <- RepairAny RepairAny)]
               suggest-relations: [(Fun RepairAny <- RepairAny RepairAny)]
               format-canonical: [(Fun String <- RepairAny)])
       identity: 'marlin/repair))

(def RepairAstPatchOpsContract
  (Record build-request: [(Fun Object <- RepairAny)]
          validate-request: [(Fun RepairBoolean <- Object)]
          from-style-finding: [(Fun Object <- RepairAny String RepairAny)]
          project-vector: [(Fun RepairVector <- Object)]))

(def RepairAstPatchContract
  (.cc (Record ast-patch: [RepairAstPatchOpsContract])
       identity: 'marlin/repair/ast-patch))
