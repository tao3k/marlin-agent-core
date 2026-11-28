;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contracts for Marlin's native Gerbil Scheme search composition boundary.
(import (only-in :clan/poo/object .cc object?)
        (only-in :clan/poo/type Exactly Fun Record Type.))
(export NativeSearchPlan SearchPlanObject SearchEngineScopeContract)

(def NativeSearchPlan (.cc Type. '.element? vector?))
(def SearchPlanObject (.cc Type. '.element? object?))

(def SearchEngineScopeContract
  (.cc (Record identity: [(Exactly 'marlin/search-engine)]
               project: [(Fun NativeSearchPlan <-)]
               project-plan: [(Fun NativeSearchPlan <- SearchPlanObject)])
       identity: 'marlin/search-engine))
