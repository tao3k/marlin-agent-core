;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Unit tests for Marlin GraphEngine rooted-vector C-ABI boundary:
;;;   1. Admitted & fail-closed dispatch vector projections (steering join guard,
;;;      detached-scope settlement denial, epoch validation);
;;;   2. Anti-split-brain fencing-epoch receipt reconciliation (stale/future/mismatch/
;;;      terminal/pending-steering convergence gate);
;;;   3. Pinned-Index + active + read-only disjoint peek Workbench tab vectors.

(import :std/test
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/graph-engine/interface GraphEngine))

(export marlin-graph-engine-c-abi-test)

(def marlin-graph-engine-c-abi-test
  (test-suite "Marlin GraphEngine rooted C-ABI vector boundary"
    (test-case "dispatch vector admits valid envelopes and rejects unsafe operations fail-closed"
      (let* ((project-dispatch (.get GraphEngine core exports project-dispatch-vector))
             (ok (project-dispatch
                   '#("req-1" "task-alpha" execute-frontier 1 project #f sqlite-turso))))
        (check (vector-length ok) => 10)
        (check (vector-ref ok 0) => 1)
        (check (vector-ref ok 1) => #t)
        (check (vector-ref ok 2) => "req-1")
        (check (vector-ref ok 3) => "task-alpha")
        (check (vector-ref ok 4) => 'execute-frontier)
        (check (vector-ref ok 5) => 1)
        (check (vector-ref ok 6) => 'project)
        (check (vector-ref ok 8) => 'sqlite-turso)
        (check (vector-ref ok 9) => #f)

        ;; Steering without join-node-id must fail closed.
        (let ((bad-steer (project-dispatch
                           '#("req-2" "task-alpha" apply-steering 2 project #f pure-memory))))
          (check (vector-ref bad-steer 1) => #f)
          (check (vector-ref bad-steer 9) => "steering-requires-join-node-id"))

        ;; Steering WITH join-node-id is admitted.
        (let ((ok-steer (project-dispatch
                          '#("req-3" "task-alpha" apply-steering 2 project "join-F" pure-memory))))
          (check (vector-ref ok-steer 1) => #t)
          (check (vector-ref ok-steer 7) => "join-F"))

        ;; Detached scope cannot commit project settlement.
        (let ((bad-detached (project-dispatch
                              '#("req-4" "task-det" commit-settlement 1 detached #f pure-memory))))
          (check (vector-ref bad-detached 1) => #f)
          (check (vector-ref bad-detached 9) => "detached-scope-cannot-commit-project-settlement"))))

    (test-case "reconcile receipt enforces monotonic fencing-epoch and C4 steering convergence"
      (let ((reconcile (.get GraphEngine core exports reconcile-receipt-vector)))
        ;; Stale fencing epoch rejected (anti-split-brain).
        (let ((stale (reconcile
                       '#("task-alpha" 3 running 0 "req-old" "task-alpha" 2 ok))))
          (check (vector-ref stale 1) => #f)
          (check (vector-ref stale 2) => 3)
          (check (vector-ref stale 4) => 'stale-fencing-epoch))

        ;; Future fencing epoch rejected.
        (let ((future (reconcile
                        '#("task-alpha" 3 running 0 "req-future" "task-alpha" 4 ok))))
          (check (vector-ref future 1) => #f)
          (check (vector-ref future 4) => 'future-fencing-epoch))

        ;; Task ID mismatch rejected.
        (let ((mismatch (reconcile
                          '#("task-alpha" 3 running 0 "req-x" "task-beta" 3 ok))))
          (check (vector-ref mismatch 1) => #f)
          (check (vector-ref mismatch 4) => 'task-id-mismatch))

        ;; Pending steering branches hold actor in waiting-steering and block settlement.
        (let ((waiting (reconcile
                         '#("task-alpha" 3 running 2 "req-3" "task-alpha" 3 ok)))
              (blocked-settle (reconcile
                                '#("task-alpha" 4 waiting-steering 1 "req-4" "task-alpha" 4 settled))))
          (check (vector-ref waiting 1) => #t)
          (check (vector-ref waiting 2) => 4)
          (check (vector-ref waiting 3) => 'waiting-steering)
          (check (vector-ref waiting 4) => 'awaiting-steering-convergence)
          (check (vector-ref blocked-settle 1) => #f)
          (check (vector-ref blocked-settle 4) => 'cannot-settle-with-pending-steering))

        ;; Zero pending steering allows settlement; subsequent receipts rejected as terminal.
        (let ((settled (reconcile
                         '#("task-alpha" 4 waiting-steering 0 "req-5" "task-alpha" 4 settled)))
              (post-term (reconcile
                           '#("task-alpha" 5 settled 0 "req-6" "task-alpha" 5 ok))))
          (check (vector-ref settled 1) => #t)
          (check (vector-ref settled 2) => 5)
          (check (vector-ref settled 3) => 'settled)
          (check (vector-ref post-term 1) => #f)
          (check (vector-ref post-term 4) => 'actor-already-terminal))))

    (test-case "workbench tabs vector pins Index at slot 0 and keeps peek tabs strictly disjoint"
      (let* ((project-tabs (.get GraphEngine core exports project-workbench-tabs-vector))
             (proj (project-tabs
                     '#("task-2"
                        #(#("task-1" "Auth" running project 2)
                          #("task-2" "Graph" waiting-steering project 4))
                        #("task-2" "task-archived-9" "task-archived-9"))))
             (tabs (vector-ref proj 2)))
        (check (vector-ref proj 0) => 1)
        (check (vector-ref proj 1) => "task-2")
        ;; 1 pinned index + 2 active tasks + 1 deduplicated disjoint peek = 4 tabs.
        (check (vector-length tabs) => 4)
        (check (vector-ref (vector-ref tabs 0) 0) => 'pinned-index)
        (check (vector-ref (vector-ref tabs 1) 1) => "task-1")
        (check (vector-ref (vector-ref tabs 1) 4) => #f)
        (check (vector-ref (vector-ref tabs 2) 1) => "task-2")
        (check (vector-ref (vector-ref tabs 2) 4) => #t)
        (check (vector-ref (vector-ref tabs 3) 0) => 'read-only-peek)
        (check (vector-ref (vector-ref tabs 3) 1) => "task-archived-9"))
      (displayln "MARLIN-GRAPH-ENGINE-C-ABI-OK"))))
