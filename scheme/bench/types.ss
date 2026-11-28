;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Record))

(export BenchScopeContract)

(def BenchScopeContract
  (.cc (Record identity: [(Exactly 'marlin/bench)]
               program-entry: [(Exactly 'marlin-agent-core/bench/mailbox-entry)]
               program-runtime-imports: [(Exactly '())])
       identity: 'marlin/bench))
