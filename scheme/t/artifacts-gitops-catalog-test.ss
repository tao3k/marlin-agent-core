;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .call .get .o .ref)
        (only-in :std/crypto/digest sha256)
        (only-in :std/encoding/hex hex-encode)
        (only-in :std/text/pregexp pregexp-replace)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
(export artifacts-gitops-catalog-test)

(def Artifacts (.get marlin-agent-core.init.exports Marlin artifacts))
(def Catalog (.get Artifacts catalog))
(def Prototypes (.get Artifacts prototypes))

(def sample-commit "0123456789abcdef0123456789abcdef01234567")
(def patch-digest (hex-encode (sha256 (string->utf8 "-old\n+new\n"))))
(def receipt-digest (hex-encode (sha256 (string->utf8 "tests passed\n"))))
(def governed-work-body
  (string-append
   ":PROPERTIES:\n"
   ":ID: work-doc\n"
   ":CONTRACT_ID: context.governance.work.v1\n"
   ":CONTRACT_REVISION: 1\n"
   ":PROJECT_ID: marlin-agent-core\n"
   ":END:\n"
   "* Active Work\n"
   ":PROPERTIES:\n"
   ":ID: sec-1\n"
   ":AGENT_ID: agent/worker-01\n"
   ":TASK_ID: task/impl-catalog\n"
   ":END:\n"
   "Governed body.\n"))
(def work-digest (hex-encode (sha256 (string->utf8 governed-work-body))))

(def (rejected? thunk)
  (with-catch (lambda (_) #t) (lambda () (thunk) #f)))

(def (rm-rf! path)
  (when (file-exists? path)
    (if (eq? (file-info-type (file-info path #f)) 'directory)
      (begin
        (for-each (lambda (name) (rm-rf! (path-expand name path)))
                  (directory-files path))
        (delete-directory path))
      (delete-file path))))

(def (call-with-temp-dir prefix proc)
  (let (dir (path-expand (string-append prefix "-" (number->string (random-integer 100000000)))
                         "/tmp"))
    (create-directory* dir)
    (dynamic-wind void
      (lambda () (proc dir))
      (lambda () (rm-rf! dir)))))

(def artifacts-gitops-catalog-test
  (test-suite "Declarative Organization Architecture and GitOps Artifacts Catalog"
    (test-case "Organization Role -> Agent capabilities, Domain scope, and SoD invariants"
      (let (admit (.get Artifacts admit-record))
        (let (ok (admit Catalog
                        (.o kind: 'governed-work-org
                            commit: sample-commit
                            worktree: "feat/gitops-artifacts"
                            digest: work-digest
                            org-body: governed-work-body
                            producer: 'worker
                            reviewer: 'architect)))
          (check (.get ok collection) => 'work)
          (check (.get ok domain) => 'worktree)
          (check (.get ok owner-role) => 'author)
          (check (.get ok reviewer-role) => 'architect))
        ;; Missing reviewer on review-required kind is refused.
        (check (rejected?
                (lambda ()
                  (admit Catalog
                         (.o kind: 'governed-work-org
                             commit: sample-commit
                             worktree: "feat/gitops-artifacts"
                             digest: work-digest
                             producer: 'worker))))
               => #t)
        ;; Self-review (architect reviewing architect) is refused by SoD.
        (check (rejected?
                (lambda ()
                  (admit Catalog
                         (.o kind: 'governed-work-org
                             commit: sample-commit
                             worktree: "feat/gitops-artifacts"
                             digest: work-digest
                             producer: 'architect
                             reviewer: 'architect))))
               => #t)
        ;; Non-review-eligible role (`author`) cannot act as reviewer.
        (check (rejected?
                (lambda ()
                  (admit Catalog
                         (.o kind: 'governed-work-org
                             commit: sample-commit
                             worktree: "feat/gitops-artifacts"
                             digest: work-digest
                             producer: 'architect
                             reviewer: 'worker))))
               => #t)
        ;; Domain confinement: `worker` (only `worktree` domain) cannot publish `shared` evidence.
        (check (rejected?
                (lambda ()
                  (admit Catalog
                         (.o kind: 'evidence-org
                             commit: sample-commit
                             worktree: "main"
                             digest: work-digest
                             producer: 'worker))))
               => #t)
        ;; Cyclic Organization Agent parent graph is refused.
        (let* ((base-org (.get Catalog organization))
               (cyclic-agents
                (.o (:: @ (.get base-org agents))
                    lead: (.o (:: @ (.get Prototypes agent))
                              identity: "agent/coordinator-00"
                              role: 'coordinator
                              parent: 'worker
                              domains: '(shared worktree verification))))
               (cyclic-catalog
                (.o (:: @ Catalog)
                    organization: (.o (:: @ base-org) agents: cyclic-agents))))
          (check (rejected?
                  (lambda ()
                    (admit cyclic-catalog
                           (.o kind: 'git-patch
                               commit: sample-commit
                               worktree: "main"
                               digest: patch-digest))))
                 => #t))))

    (test-case "POO refinement extends Roles, Agents, Domains, Collections, and Kinds declaratively"
      (let* ((admit (.get Artifacts admit-record))
             (project-org (.get Artifacts project-context-org))
             (inspect-org (.get Artifacts inspect-context-org))
             (base-org (.get Catalog organization))
             (ext-org
              (.o (:: @ base-org)
                  roles: (.o (:: @ (.get base-org roles))
                             auditor: (.o (:: @ (.get Prototypes role))
                                          identity: "role/security-auditor"
                                          title: "Security Auditor"
                                          capabilities: '(record-audit review-org)
                                          review-eligible?: #t))
                  agents: (.o (:: @ (.get base-org agents))
                              lead: (.o (:: @ (.get (.get base-org agents) lead))
                                        domains: '(shared worktree verification security))
                              sec-auditor: (.o (:: @ (.get Prototypes agent))
                                               identity: "agent/sec-01"
                                               role: 'auditor
                                               parent: 'lead
                                               domains: '(security)))))
             (ext-catalog
              (.o (:: @ Catalog)
                  identity: "marlin-sec-artifacts"
                  organization: ext-org
                  domains: (.o (:: @ (.get Catalog domains))
                               security: (.o (:: @ (.get Prototypes domain))
                                             identity: 'security
                                             path: "security"
                                             worktree-scope: 'project-shared))
                  collections: (.o (:: @ (.get Catalog collections))
                                   audits: (.o (:: @ (.get Prototypes collection))
                                               identity: 'audits
                                               domain: 'security
                                               directory: "security/audits"
                                               index-file: "audits.org"
                                               contract-id: "context.security.audit.v1"
                                               allowed-kinds: '(security-audit)))
                  kinds: (.o (:: @ (.get Catalog kinds))
                             security-audit: (.o (:: @ (.get Prototypes kind))
                                                 kind: 'security-audit
                                                 media: 'blob
                                                 collection: 'audits
                                                 required-capability: 'record-audit
                                                 review-required?: #t
                                                 contract-id: "context.security.audit.v1"))))
             (rec (admit ext-catalog
                         (.o kind: 'security-audit
                             commit: sample-commit
                             worktree: "main"
                             digest: receipt-digest
                             producer: 'sec-auditor
                             reviewer: 'lead)))
             (org-text (project-org ext-catalog (list rec)))
             (inspected (inspect-org ext-catalog org-text)))
        (check (rejected?
                (lambda ()
                  (admit ext-catalog
                         (.o kind: 'security-audit
                             commit: sample-commit
                             worktree: "main"
                             digest: receipt-digest
                             producer: 'sec-auditor
                             reviewer: 'architect))))
               => #t)
        (check (.get rec collection) => 'audits)
        (check (.get rec domain) => 'security)
        (check (.get inspected valid?) => #t)
        (check (.get inspected role-count) => 6)
        (check (.get inspected agent-count) => 6)
        (check (.get inspected collection-count) => 6)))

    (test-case "GitOps Artifact Catalog round-trips through project-context-org / inspect-context-org and catches tampering"
      (let* ((admit (.get Artifacts admit-record))
             (project-org (.get Artifacts project-context-org))
             (inspect-org (.get Artifacts inspect-context-org))
             (r1 (admit Catalog
                        (.o kind: 'git-patch
                            commit: sample-commit
                            worktree: "wt-a"
                            digest: patch-digest
                            producer: 'worker)))
             (r2 (admit Catalog
                        (.o kind: 'governed-work-org
                            commit: sample-commit
                            worktree: "wt-a"
                            digest: work-digest
                            org-body: governed-work-body
                            producer: 'worker
                            reviewer: 'architect)))
             (org-text (project-org Catalog (list r1 r2)))
             (inspected (inspect-org Catalog org-text)))
        (check (.get inspected valid?) => #t)
        (check (.get inspected role-count) => 5)
        (check (.get inspected agent-count) => 5)
        (check (.get inspected collection-count) => 5)
        (check (length (.get inspected artifacts)) => 2)
        (check (rejected?
                (lambda ()
                  (inspect-org Catalog
                               (pregexp-replace ":COLLECTION: patches" org-text ":COLLECTION: receipts"))))
               => #t)
        (check (rejected?
                (lambda ()
                  (inspect-org Catalog
                               (pregexp-replace ":REVIEWER_KEY: architect" org-text ":REVIEWER_KEY: worker"))))
               => #t)))

    (test-case "materialize-artifact-directory! emits multi-file GitOps hierarchy with data/ passing Orgize org-directory-validate"
      (call-with-temp-dir "marlin-gitops-artifacts-dir"
        (lambda (dir)
          (let* ((admit (.get Artifacts admit-record))
                 (materialize! (.get Artifacts materialize-artifact-directory!))
                 (r1 (admit Catalog
                            (.o kind: 'git-patch
                                commit: sample-commit
                                worktree: "wt-a"
                                digest: patch-digest
                                producer: 'worker)))
                 (r2 (admit Catalog
                            (.o kind: 'test-receipt
                                commit: sample-commit
                                worktree: "wt-a"
                                digest: receipt-digest
                                producer: 'verifier)))
                 (result (materialize! Catalog (list r1 r2) dir))
                 (validation (.get result validation)))
            (check (.get validation complete?) => #t)
            (check (.get validation passed?) => #t)
            (check (file-exists? (path-expand "context.org" dir)) => #t)
            (check (file-exists? (path-expand "data" dir)) => #t)
            (check (file-exists? (path-expand "shared/topology/topology.org" dir)) => #t)
            (check (file-exists? (path-expand "worktrees/work/work.org" dir)) => #t)
            (check (file-exists? (path-expand "worktrees/patches/patches.org" dir)) => #t)
            (check (file-exists? (path-expand "verification/receipts/receipts.org" dir)) => #t)))))

    (test-case "artifacts-exchange persists to Lab durable journal and restores deterministic context.org and commit-ordered digests"
      (call-with-temp-dir "marlin-gitops-artifacts-journal"
        (lambda (dir)
          (let* ((journal-path (path-expand "context.db" dir))
                 (exchange (.get Artifacts exchange))
                 (p1 (exchange (vector 'record journal-path 'git-patch sample-commit "wt-a" patch-digest)))
                 (p2 (exchange (vector 'record journal-path 'test-receipt sample-commit "wt-a" receipt-digest)))
                 (p3 (exchange (vector 'record-governed journal-path
                                       (.o kind: 'governed-work-org
                                           commit: sample-commit
                                           worktree: "wt-a"
                                           digest: work-digest
                                           org-body: governed-work-body
                                           producer: 'worker
                                           reviewer: 'architect))))
                 (restored (exchange (vector 'restore journal-path))))
            (check (vector-ref p1 1) => (vector patch-digest))
            (check (vector-ref p2 1) => (vector patch-digest receipt-digest))
            (check (vector-ref p3 1) => (vector patch-digest receipt-digest work-digest))
            (check (vector-ref restored 0) => (vector-ref p3 0))
            (check (vector-ref restored 1) => (vector patch-digest receipt-digest work-digest)))))

    (test-case "artifacts-exchange projects declarative workspace storage and session layout"
      (let* ((exchange (.get Artifacts exchange))
             (layout (exchange (vector 'workspace-layout))))
        (check (vector-length layout) => 7)
        (check (vector-ref layout 0) => "session.db")
        (check (vector-ref layout 1) => "context.db")
        (check (vector-ref layout 2) => "search")
        (check (vector-ref layout 3) => ".processes.json")
        (check (vector-ref layout 4) => "project:marlin-agent-core")
        (check (vector-ref layout 5) => "session:default")
        (check (vector-ref layout 6) => "agent/host-operator"))))))
