;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in "../context-system/worktree.ss" prepare-worktree-context deliver-worktree-context))
(export git-worktree-context-test)

(def Index
  ":PROPERTIES:\n:ID: index\n:CONTRACT_ID: context.index.v1\n:CONTRACT_REVISION: 1\n:END:\n")
(def Topology
  ":PROPERTIES:\n:ID: topology\n:CONTRACT_ID: context.topology.v1\n:CONTRACT_REVISION: 1\n:END:\n")

(def git-worktree-context-test
  (test-suite "Git worktree Context admission"
    (test-case "Owner and cut are checked before source admission"
      (check (prepare-worktree-context '#("/trusted" "/other" "checkout" "commit")) => #f)
      (let (permit (prepare-worktree-context '#("/trusted" "/trusted" "checkout" "commit")))
        (check (procedure? permit) => #t)
        (check (deliver-worktree-context (vector permit "other" Index Topology)) => #f)
        (let (catalog (deliver-worktree-context (vector permit "commit" Index Topology)))
          (check (vector-length catalog) => 4)
          (check (vector-ref (vector-ref catalog 2) 2) => "context.index.v1")
          (check (vector-ref (vector-ref catalog 3) 2) => "context.topology.v1"))))))
