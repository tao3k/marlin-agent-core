;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validated core scope publication for Marlin's modular `repair` module.
(import (only-in :clan/poo/mop .new/checked)
        "./types.ss"
        "./objects.ss"
        "./funcs.ss")
(export RepairScope)

(def RepairScope
  (.new/checked RepairScopeContract
    identity: 'marlin/repair
    policy: DefaultRepairPolicyValue
    read-and-repair: (lambda (text registry)
                       (let-values (((form tags) (repair-and-read-ascent-query text registry)))
                         (vector form tags)))
    repair-ast: (lambda (form registry)
                  (let-values (((repaired tags) (repair-ascent-ast form registry)))
                    (vector repaired tags)))
    suggest-relations: suggest-similar-relations
    format-canonical: format-canonical-search-form))
