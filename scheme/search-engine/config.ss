;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validated publication for Marlin's native search composition module.
(import (only-in :clan/poo/mop .new/checked)
        "./types.ss" "./objects.ss" "./funcs.ss")
(export SearchEngineScope)

(def SearchEngineScope
  (.new/checked SearchEngineScopeContract
    identity: 'marlin/search-engine
    project: (lambda () (project-search-plan DefaultSearchPlanValue))
    project-plan: (lambda (plan) (project-search-plan plan))))
