;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Fun List Record Symbol Type.))
(export UiCentaurTabsScopeContract)

(def UiTabBarValue (.cc Type. '.element? vector?))
(def UiCentaurTabsScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-centaur-tabs)]
               bar-for: [(Fun UiTabBarValue <- (List Symbol) Symbol)])
       identity: 'marlin/ui-centaur-tabs))
