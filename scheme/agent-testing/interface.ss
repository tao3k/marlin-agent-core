;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative Agent Testing Module selected by Yuanxu.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" AgentTestingScope)
        (only-in "./types.ss" AgentTestingScopeContract))
(export AgentTesting)

(.def AgentTesting
  identity: 'agent-testing
  core: (.o scope-contract: AgentTestingScopeContract
            exports: AgentTestingScope))
