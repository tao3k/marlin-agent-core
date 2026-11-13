;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Record)
        (only-in :clan/poo/mop Fun Object Type.))
(export UiLayoutScopeContract)

(def UiBarValue (.cc Type. '.element? vector?))
(def UiLayoutScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-layout)] policy-for: [(Fun Object <- UiBarValue)])
       identity: 'marlin/ui-layout))
