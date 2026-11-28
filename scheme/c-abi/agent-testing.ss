;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme C-ABI exports for Marlin's declarative Agent Testing module
;;; (POO scenario suites, read-only Cedar governance, and Bi-Temporal Causality
;;; trajectory verification). Zero JSON across the native boundary.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/agent-testing/config AgentTestingScope))
(export marlin-agent-testing-project-suite
        marlin-agent-testing-verify-trajectory)

(def (exception->diagnostic-string err)
  (call-with-output-string
    (lambda (port)
      (display-exception err port))))

(def (marlin-agent-testing-project-suite input)
  (with-catch
    (lambda (err)
      (vector 'testing-error (exception->diagnostic-string err)))
    (lambda ()
      ((.get AgentTestingScope project-suite) input))))

(def (marlin-agent-testing-verify-trajectory request)
  (with-catch
    (lambda (err)
      (vector 'testing-error (exception->diagnostic-string err)))
    (lambda ()
      ((.get AgentTestingScope verify-trajectory) request))))

(begin-foreign
  (c-define (agent-testing-project-suite root) (int64) int64
    "marlin_agent_testing_project_suite" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/agent-testing#marlin-agent-testing-project-suite))
  (c-define (agent-testing-verify-trajectory root) (int64) int64
    "marlin_agent_testing_verify_trajectory" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/agent-testing#marlin-agent-testing-verify-trajectory)))
