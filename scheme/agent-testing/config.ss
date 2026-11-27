;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative Agent Testing scope binding POO suites and trajectory verification.
(import (only-in :clan/poo/mop .new/checked)
        "./types.ss" "./objects.ss" "./funcs.ss"
        "../search-dsl/evidence-chain.ss")
(export AgentTestingScope)

(def AgentTestingScope
  (.new/checked AgentTestingScopeContract
    identity: 'marlin/agent-testing
    project-suite: (lambda (selector) (project-testing-suite selector))
    verify-trajectory: verify-testing-trajectory))
