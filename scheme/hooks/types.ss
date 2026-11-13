;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contract for the hook decision publication.

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Bool)
        (only-in :clan/poo/type Record String OrFalse))

(export HooksScopeContract)

(def HookSelection
  (Record kind: [String] source: [String] selector: [(OrFalse String)]
          matched: [Bool] matched-case: [(OrFalse String)] dynamic-hook-action: [(OrFalse Object)]))

(def HooksScopeContract
  (.cc (Record select: [(Fun HookSelection <- (OrFalse Object) String Object Object String String)])
       identity: 'marlin/hooks))
