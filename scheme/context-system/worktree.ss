;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o .ref)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/objects AiAgenticContextScope.)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/funs-scope
                 agentic-ai-lab-ai-agentic-context-digest)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/query-provider
                 context-org-source context-query-host context-query-source!)
        :tao3k/agentic-ai-lab/modules/ai-agentic-context/use-host
        :tao3k/agentic-ai-lab/modules/temporal-causality/truth-maintenance/support/policy
        :tao3k/agentic-ai-lab/modules/temporal-causality/truth-maintenance/support/policy-host
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/time/objects
                 agentic-ai-lab-temporal-instant))
(export prepare-worktree-context deliver-worktree-context revoke-worktree-context)

(def (clock coordinate)
  (agentic-ai-lab-temporal-instant (number->string coordinate) "git-context" coordinate "marlin" 'observed))

(def (source-id project worktree commit file)
  (string-append
   (agentic-ai-lab-ai-agentic-context-digest ['git-worktree-source project worktree commit file])
   "/" file))

(def (sources-authorized? authority policies entries)
  (andmap
   (match <> ([file id]
     (let (decision (agentic-ai-lab-context-use-host-observe authority policies file 1 'display))
       (and (eq? (.ref decision 'decision) 'allowed)
            (equal? (.ref (.ref decision 'grant) 'admission-digest) id)))))
   entries))

(def (revoke-sources! authority entries)
  (for-each
   (match <> ([file _]
     (let (grant (agentic-ai-lab-context-use-host-current authority file))
       (when (.ref grant 'enabled?)
         (agentic-ai-lab-context-use-host-refresh! authority
           (agentic-ai-lab-context-use-grant-revoke grant))))))
   entries))

;; The host resolves the operator's configured repository and the observed
;; owner independently. Scheme issues display grants before any blob is read.
(def (prepare-worktree-context input)
  (unless (and (vector? input) (= (vector-length input) 4))
    (error "Invalid Git Context authorization request"))
  (let ((trusted (vector-ref input 0)) (owner (vector-ref input 1))
        (checkout (vector-ref input 2)) (commit (vector-ref input 3)))
    (and (equal? trusted owner)
         (let* ((scope (.o (:: @ AiAgenticContextScope.) bundle: "marlin" organization: "local"
                           epoch: 1 project: owner worktree: checkout source-cut: commit
                           actor: "operator" task: "read-git-context" destination: "tui"))
                (authority (agentic-ai-lab-context-use-host))
                (policies (agentic-ai-lab-temporal-support-policy-host))
                (policy (agentic-ai-lab-temporal-support-policy "git-context" "1" (clock 1) (clock 10)))
                (contract (agentic-ai-lab-context-use-contract "operator" "read-git-context" "operator-worktree" '() '() #t))
                (entries (map (lambda (file) [file (source-id owner checkout commit file)])
                              '("index.org" "topology.org"))))
           (agentic-ai-lab-temporal-support-policy-host-refresh! policies policy 1 (clock 1))
           (for-each
            (match <> ([file id]
              (agentic-ai-lab-context-use-host-refresh! authority
                (agentic-ai-lab-context-use-grant file 1 "operator-worktree"
                  id
                  contract "git-context" (.ref policy 'semantic-digest) '(display) #t))))
            entries)
           (and (sources-authorized? authority policies entries)
                (lambda (operation body)
                  (case operation
                    ((deliver)
                     (and (equal? commit (vector-ref body 1))
                          (sources-authorized? authority policies entries)
                          (publish-worktree-sources scope entries body)))
                    ((revoke) (revoke-sources! authority entries))
                    (else (error "Invalid Git Context permit operation")))))))))

(def (publish-worktree-sources scope entries body)
  ;; Validate both payloads before admitting either. Only the completed
  ;; catalog and immutable receipts escape.
  (let* ((sources (map (lambda (entry position)
                         (context-org-source (cadr entry) 1 (vector-ref body position)))
                       entries '(2 3)))
         (catalog (context-query-host))
         (receipts (map (lambda (source)
                          (let (cut (context-query-source! catalog source))
                            (vector (.ref cut 'identity) (.ref cut 'source-digest)
                                    (.ref cut 'contract-identity) (.ref cut 'contract-revision))))
                        sources)))
    (apply vector catalog scope receipts)))

(def (deliver-worktree-context input)
  (unless (and (vector? input) (= (vector-length input) 4) (procedure? (vector-ref input 0)))
    (error "Invalid Git Context delivery request"))
  ((vector-ref input 0) 'deliver input))

(def (revoke-worktree-context permit)
  (unless (procedure? permit)
    (error "Invalid Git Context permit"))
  (permit 'revoke #f))
