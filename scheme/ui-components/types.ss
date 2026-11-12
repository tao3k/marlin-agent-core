;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Exactly Record)
        (only-in "./commands.ss" marlin-ui-command-entries?))
(export UiComponentsScopeContract)

(def UiCommandEntries (.cc Type. '.element? marlin-ui-command-entries?))

(def UiComponentsScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-components)]
               components: [Object]
               contribution-node: [(Fun Object <- Object)]
               extension-node: [(Fun Object <- Object)]
               guide-node: [(Fun Object <-)]
               command-catalog: [Object]
               command-entries: [(Fun UiCommandEntries <- Object)]
               browse-command: [(Fun Object <- UiCommandEntries Object)])
       identity: 'marlin/ui-components))
