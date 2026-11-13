;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Exactly List Record))
(export UiModelineScopeContract)

(def UiProfileIdentity (.cc Type. '.element? symbol?))
(def UiBarValue (.cc Type. '.element? vector?))

(def UiModelineScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-modeline)]
               nodes: [(Fun (List Object) <-)]
               nodes-for: [(Fun (List Object) <- UiProfileIdentity)]
               bar-for: [(Fun UiBarValue <- (List Object))])
       identity: 'marlin/ui-modeline))
