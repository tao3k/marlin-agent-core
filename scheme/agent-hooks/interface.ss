;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Hook decisions published by Marlin.

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" AgentHooksScope)
        (only-in "./types.ss" AgentHooksScopeContract))
(export AgentHooks)

(.def AgentHooks
  identity: 'agent-hooks
  core: (.o scope-contract: AgentHooksScopeContract
            exports: AgentHooksScope))
