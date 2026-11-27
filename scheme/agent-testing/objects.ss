;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Reusable POO prototypes for declarative Agent Testing Scenarios and Suites.
;;;
;;; Concrete test suites belong in dedicated test-case modules (e.g.
;;; `test-case-one.ss`, `test-case-two.ss`), never inside `objects.ss`.
;;; Scenarios send ONLY a pure natural-language user `message` (zero coaching
;;; system prompt) so tool selection and multi-tier search convergence are
;;; evaluated honestly from tool declarations alone within `< 5` rounds.
(import (only-in :clan/poo/object .def)
        (only-in "./governance.ss" ReadOnlySearchTestingGovernanceValue))
(export TestingScenario.
        TestingSuite.)

(.def TestingScenario.
  id: 'unnamed-scenario
  title: ""
  message: ""
  target-max-rounds: 4
  max-rounds: 4
  max-tool-calls: 20
  require-workspace-search?: #t
  enable-reflection?: #f
  required-tiers: '()
  governance: ReadOnlySearchTestingGovernanceValue
  checkpoints: '())

(.def TestingSuite.
  id: 'marlin-agent-testing-suite
  scenarios: '())
