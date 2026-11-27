;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :marlin-agent-core/agent-testing/funcs
                 project-testing-suite
                 verify-testing-trajectory))
(export agent-testing-test)

(def (ascii-printable-english? s)
  (and (string? s)
       (> (string-length s) 20)
       (let loop ((i 0))
         (if (>= i (string-length s))
           #t
           (let ((code (char->integer (string-ref s i))))
             (and (<= code 8212) ;; ASCII + standard punctuation (e.g. em-dash)
                  (loop (+ i 1))))))))

(def agent-testing-test
  (test-suite "Declarative Agent Testing, Read-Only Governance & Temporal Causality"

    (test-case "Test Case One projects 11-slot message-only English scenarios with <5 max-rounds and strict Cedar governance"
      (let* ((projected (project-testing-suite))
             (tag (vector-ref projected 0))
             (suite-id (vector-ref projected 1))
             (scenarios (vector-ref projected 2))
             (first (vector-ref scenarios 0))
             (gov (vector-ref first 7))
             (cedar (vector-ref gov 6)))
        (check tag => 'testing-suite)
        (check suite-id => 'marlin-search-read-only-eval)
        (check (vector-length scenarios) => 4)
        (check (vector-ref first 0) => 'model_gateway_implementations)
        ;; 11-slot scenario vector (zero system-prompt slot; includes required-tiers).
        (check (vector-length first) => 11)
        (check (ascii-printable-english? (vector-ref first 2)) => #t)
        ;; Strict <5 round budget across every scenario in Test Case One.
        (check (< (vector-ref first 3) 5) => #t)
        (check (< (vector-ref first 4) 5) => #t)
        ;; Strict read-only search governance: write-allowed? = #f, 3 read-only search tools allowed, bash denied.
        (check (vector-ref gov 0) => 'read-only-search-governance)
        (check (vector-ref gov 1) => #f)
        (check (vector-ref gov 4) => '#(workspace_search structural_query constraint_solve))
        (check (vector-ref gov 5) => '#(bash))
        (check (>= (vector-length cedar) 4) => #t)
        (check (vector-ref first 9) => #f)
        (check (vector-ref first 10) => '#())))

    (test-case "Test Case Two projects 4 complex English scenarios with <5 max-rounds, reflection, and receipt tier assertions"
      (let* ((projected (project-testing-suite 'complex))
             (tag (vector-ref projected 0))
             (suite-id (vector-ref projected 1))
             (scenarios (vector-ref projected 2))
             (first (vector-ref scenarios 0))
             (fourth (vector-ref scenarios 3))
             (gov (vector-ref first 7)))
        (check tag => 'testing-suite)
        (check suite-id => 'marlin-complex-coding-read-only-eval)
        (check (vector-length scenarios) => 4)
        (check (vector-ref first 0) => 'cross_layer_search_dsl_change_impact)
        (check (vector-length first) => 11)
        (check (ascii-printable-english? (vector-ref first 2)) => #t)
        (check (< (vector-ref first 3) 5) => #t)
        (check (< (vector-ref first 4) 5) => #t)
        (check (vector-ref gov 1) => #f)
        (check (vector-ref first 9) => #t)
        (check (vector-ref fourth 10) => '#(L1-Ascent L2-Dusa))))

    (test-case "Bi-Temporal Causality admits a multi-round read-only search trajectory with repeated kinds and denied write attempt"
      (let* ((events
               (vector
                 #("ev-1" prompt 1 #() 1 1)
                 #("ev-2" turn 2 #("ev-1") 2 2)
                 #("ev-3" gov_forbid 3 #("ev-2") 3 3)
                 #("ev-4" tool_denied 4 #("ev-3") 4 4)
                 #("ev-5" gov_permit 5 #("ev-2") 5 5)
                 #("ev-6" tool_search_ok 6 #("ev-5") 6 6)
                 #("ev-7" turn 7 #("ev-4" "ev-6") 7 7)
                 #("ev-8" gov_permit 8 #("ev-7") 8 8)
                 #("ev-9" tool_search_ok 9 #("ev-8") 9 9)
                 #("ev-10" final_answer 10 #("ev-9") 10 10)))
             (verdict (verify-testing-trajectory
                        (vector 'trajectory-request "model_gateway_implementations" #f events))))
        (check (vector-ref verdict 0) => 'trajectory-verdict)
        (check (vector-ref verdict 1) => #t)
        (check (vector-ref verdict 2) => 'complete)
        (check (vector-ref verdict 3) => 'valid)
        (check (vector-ref verdict 4) => 10)
        (check (vector-length (vector-ref verdict 5)) => 0)
        (check (and (string? (vector-ref verdict 6))
                    (> (string-length (vector-ref verdict 6)) 10))
               => #t)))

    (test-case "Negative control: Bi-Temporal Causality rejects a trajectory containing executed write tool call"
      (let* ((events
               (vector
                 #("ev-1" prompt 1 #() 1 1)
                 #("ev-2" turn 2 #("ev-1") 2 2)
                 #("ev-3" tool_write_executed 3 #("ev-2") 3 3)
                 #("ev-4" final_answer 4 #("ev-3") 4 4)))
             (verdict (verify-testing-trajectory
                        (vector 'trajectory-request "model_gateway_implementations" #f events))))
        (check (vector-ref verdict 0) => 'trajectory-verdict)
        (check (vector-ref verdict 1) => #f)
        (check (vector-ref verdict 5) => '#(tool_write_executed))))))
