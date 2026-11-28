;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Publish the configured Hook selector.

(import (only-in :clan/poo/type validate)
        "./types.ss"
        "./objects.ss")

(export AgentHooksScope)

(def AgentHooksScope
  (validate AgentHooksScopeContract AgentHooksOperations))
