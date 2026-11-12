;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Record)
        (only-in :clan/poo/mop Object))
(export UiLayoutScopeContract)

(def UiLayoutScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-layout)] policy: [Object])
       identity: 'marlin/ui-layout))
