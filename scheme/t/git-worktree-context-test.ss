;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .ref)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/query-provider
                 context-query-current-source context-query-export)
        (only-in "../context-system/worktree.ss"
                 prepare-worktree-context deliver-worktree-context revoke-worktree-context))
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
          (check (vector-ref (vector-ref catalog 3) 2) => "context.topology.v1"))))
    (test-case "Revoked delivery leaves the retained Lab catalog and cached receipts intact"
      (let* ((current-request '#("/trusted" "/trusted" "checkout" "commit"))
             (late-request '#("/trusted" "/trusted" "other-checkout" "next-commit"))
             (current (prepare-worktree-context current-request))
             (late (prepare-worktree-context late-request))
             (cached (deliver-worktree-context (vector current "commit" Index Topology)))
             (catalog (vector-ref cached 0))
             (index-receipt (vector-ref cached 2))
             (topology-receipt (vector-ref cached 3)))
        (revoke-worktree-context late)
        (revoke-worktree-context late)
        (check (deliver-worktree-context (vector late "next-commit" Index Topology)) => #f)
        ;; The admission fence runs before even validating either Org payload.
        (check (deliver-worktree-context (vector late "next-commit" #f #f)) => #f)
        (let* ((fresh (prepare-worktree-context late-request))
               (accepted (deliver-worktree-context (vector fresh "next-commit" Index Topology)))
               (index-id (vector-ref (vector-ref accepted 2) 0))
               (topology-id (vector-ref (vector-ref accepted 3) 0)))
          (check (context-query-current-source catalog index-id #f) => #f)
          (check (context-query-current-source catalog topology-id #f) => #f)
          (check (length (.ref (context-query-export catalog) 'history)) => 2)
          (check (.ref (context-query-current-source catalog (vector-ref index-receipt 0))
                       'source-digest) => (vector-ref index-receipt 1))
          (check (.ref (context-query-current-source catalog (vector-ref topology-receipt 0))
                       'source-digest) => (vector-ref topology-receipt 1)))))))
