;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Record))
(export UiTuiScopeContract)
(def UiTuiScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-tui)]
               program-entry: [(Exactly 'marlin-agent-core/ui-tui/entry)])
       identity: 'marlin/ui-tui))
