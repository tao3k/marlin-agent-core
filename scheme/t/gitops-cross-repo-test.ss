;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :std/misc/ports read-all-as-string read-all-as-lines)
        (only-in :std/misc/process run-process)
        (only-in :std/list/list find)
        (only-in :std/crypto/digest sha256)
        (only-in :std/encoding/hex hex-encode)
        (only-in :clan/poo/object .o .ref)
        (only-in :tao3k/yuanxu/observability/testing-case yuanxu-test-case)
        (only-in :orgize/languages/org/modules/org-elements/source-interface
                 org-source-element-graph org-source-element-field)
        (only-in :orgize/languages/org/modules/org-elements/values org-element-graph-records)
        (only-in :orgize/languages/org/modules/org-contract/config OrgDirectoryProfile.)
        (only-in :orgize/languages/org/modules/org-contract/directory-runtime org-directory-validate)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/objects AiAgenticContextScope.)
        :tao3k/agentic-ai-lab/modules/ai-agentic-context/task-publication
        :tao3k/agentic-ai-lab/modules/ai-agentic-context/query-provider
        :tao3k/agentic-ai-lab/modules/ai-agentic-context/use-host
        (only-in :tao3k/agentic-ai-lab/modules/search-engine/interface
                 agentic-ai-lab-search-factor-observation)
        :tao3k/agentic-ai-lab/modules/temporal-causality/truth-maintenance/support/policy
        :tao3k/agentic-ai-lab/modules/temporal-causality/truth-maintenance/support/policy-host
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/time/objects
                 agentic-ai-lab-temporal-instant))

(export gitops-cross-repo-test gitops-inflight-test)

(def IndexContract "marlin.fixture.gitops.index.v1")
(def BranchContract "marlin.fixture.gitops.branch.v1")
(def WorktreeContract "context.gitops.worktree.worktrunk.v1")
(def BranchPath "branches/review/topology.org")
(def (property row name)
  (org-source-element-field row (string-append "property:" name)))

(defstruct git-object (relative route common-dir git-dir worktree ref base generation publication oid digest))
(defstruct evidence (cut records object))

(def (fixture-path name)
  (string-append (getenv "MARLIN_GITOPS_FIXTURE") "/" name))

(def (read-object-metadata name relative)
  (match (call-with-input-file (fixture-path (string-append name ".git")) read-all-as-lines)
    ([route common-dir git-dir worktree ref base generation publication oid digest]
     (make-git-object relative route common-dir git-dir worktree ref base
                      (string->number generation) publication oid digest))))

(def (source-id object)
  (string-append (git-object-route object) "/" (git-object-ref object) "/" (git-object-relative object)))

(def (decode-evidence object bytes)
  (make-evidence
   (context-org-source (source-id object) (git-object-generation object) bytes)
   (org-element-graph-records (org-source-element-graph bytes))
   object))

(def (load-evidence name relative)
  (decode-evidence (read-object-metadata name relative)
                   (call-with-input-file (fixture-path (string-append name ".org")) read-all-as-string)))

(def (headline evidence id)
  (find (lambda (row) (equal? (property row "ID") id)) (evidence-records evidence)))

(def (bound? record contract)
  (let ((binding (headline record "binding")) (cut (evidence-cut record))
        (object (evidence-object record)))
    (and (equal? (.ref cut 'contract-identity) contract)
         (equal? (.ref cut 'contract-revision) "1")
         (equal? (.ref cut 'source-digest) (git-object-digest object))
         (equal? (property binding "CUSTOM_ID") "binding")
         (equal? (map (lambda (name) (property binding name))
                      '("GITOPS_PROJECT" "GITOPS_COMMON_DIR" "GITOPS_GIT_DIR"
                        "GITOPS_WORKTREE" "GITOPS_REF" "GITOPS_SOURCE_CUT" "SOURCE_GENERATION"))
                 (list (git-object-route object) (git-object-common-dir object)
                       (git-object-git-dir object) (git-object-worktree object)
                       (git-object-ref object) (git-object-base object)
                       (number->string (git-object-generation object)))))))

;; This port checks the observed bindings and the declared origin before
;; handing the two cuts to Lab's publication host.
(def (related? index branch)
  (let ((relation (headline index "related")) (target (evidence-object branch)))
    (and (bound? index IndexContract)
         (bound? branch BranchContract)
         (not (equal? (git-object-common-dir (evidence-object index)) (git-object-common-dir target)))
         (equal? (map (lambda (name) (property relation name))
                      '("ORIGIN_ROUTE" "ORIGIN_REF" "ORIGIN_PATH" "ORIGIN_ANCHOR"
                        "ORIGIN_PUBLICATION" "ORIGIN_BLOB" "ORIGIN_GENERATION"))
                 (list (git-object-route target) (git-object-ref target) (git-object-relative target)
                       "binding" (git-object-publication target) (git-object-oid target)
                       (number->string (git-object-generation target)))))))

(def (at coordinate)
  (agentic-ai-lab-temporal-instant
   (number->string coordinate) "clock" coordinate "host" 'observed))
(def Policy (agentic-ai-lab-temporal-support-policy "org-policy" "1" (at 1) (at 10)))
(def Contract (agentic-ai-lab-context-use-contract "actor" "read-context" "task" '() '() #t))
(defstruct publication (scope queries grants policies host))

(def (make-context index)
  (let* ((scope (.o (:: @ AiAgenticContextScope.) bundle: "lab" organization: "context"
                    epoch: 1 project: (git-object-route index) worktree: (git-object-worktree index)
                    source-cut: (git-object-base index) actor: "actor" task: "read-context" destination: "search"))
         (queries (context-query-host))
         (grants (agentic-ai-lab-context-use-host))
         (policies (agentic-ai-lab-temporal-support-policy-host)))
    (agentic-ai-lab-temporal-support-policy-host-refresh! policies Policy 1 (at 1))
    (make-publication scope queries grants policies (context-task-publication-host scope queries grants policies))))

(def (grant! context grant record anchor)
  (agentic-ai-lab-context-use-host-refresh!
   (publication-grants context)
   (agentic-ai-lab-context-use-grant
    grant 1 "manifest" (context-task-read-scope (publication-scope context) (source-id record) anchor)
    Contract "org-policy" (.ref Policy 'semantic-digest) '(display) #t)))

(def (revoke! context grant)
  (let* ((grants (publication-grants context))
         (active (agentic-ai-lab-context-use-host-current grants grant)))
    (agentic-ai-lab-context-use-host-refresh! grants (agentic-ai-lab-context-use-grant-revoke active))))

;; Fixture-only pre-open port. Both grants must cover the exact task scope,
;; source and anchor before this port can spawn Git. The saved stamps let the
;; tests revoke a grant between preparation and the attempted consumption.
(def (display-stamp context id object anchor)
  (let* ((grants (publication-grants context))
         (scope (publication-scope context))
         (grant (agentic-ai-lab-context-use-host-current grants id #f)))
    (and grant
         (let* ((decision (agentic-ai-lab-context-use-host-observe
                           grants (publication-policies context) id (.ref grant 'generation) 'display))
                (contract (.ref grant 'contract)))
           (and (eq? (.ref decision 'decision) 'allowed)
                (equal? (.ref contract 'actor) (.ref scope 'actor))
                (equal? (.ref contract 'task) (.ref scope 'task))
                (equal? (.ref grant 'admission-digest)
                        (context-task-read-scope scope (source-id object) anchor))
                (list (.ref grant 'semantic-digest)
                      (.ref (.ref decision 'policy-snapshot) 'generation)))))))

(def (pair-stamp context index branch source-grant target-grant)
  (let ((source (display-stamp context source-grant index "related"))
        (target (display-stamp context target-grant branch "binding")))
    (and source target (list source target))))

(def (trace-path attempt side kind)
  (fixture-path (string-append "preopen-" attempt "-" side "." kind)))

;; Git's batch child stays open until the caller drains the object and
;; closes stdin. The continuation runs after the first payload character
;; while the remainder has yet to cross the fixture's delivery boundary.
(def (call-with-started-blob object attempt side receive)
  (run-process
   (list "env"
         (string-append "GIT_TRACE2_EVENT=" (trace-path attempt side "process"))
         (string-append "GIT_TRACE_PACK_ACCESS=" (trace-path attempt side "pack"))
         "git" "--no-replace-objects" "--no-lazy-fetch"
         (string-append "--git-dir=" (git-object-common-dir object))
         "cat-file" "--batch")
   coprocess:
   (lambda (port)
     (display (git-object-oid object) port)
     (newline port)
     (force-output port)
     (match (string-split (read-line port) #\space)
       ([oid "blob" size]
        (let (first (read-char port))
          (unless (and (equal? oid (git-object-oid object)) (char? first))
            (error "Git did not start the requested blob" side oid))
          (receive
           (lambda ()
             (close-output-port port)
             (let* ((framed (string-append (string first) (read-all-as-string port)))
                    (end (- (string-length framed) 1))
                    (payload (substring framed 0 end))
                    (bytes (string->utf8 payload)))
               (unless (and (char=? (string-ref framed end) #\newline)
                            (= (u8vector-length bytes) (string->number size))
                            (equal? (string-append "sha256:" (hex-encode (sha256 bytes)))
                                    (git-object-digest object)))
                 (error "Git returned an unexpected blob payload" side oid))
               payload)))))))))

(def (read-protected-pair context index branch source-grant target-grant prepared attempt
                          (before-delivery void))
  (def (current?) (equal? prepared (pair-stamp context index branch source-grant target-grant)))
  (and prepared
       (current?)
       (match (call-with-started-blob index attempt "a"
                (lambda (finish-source)
                  (call-with-started-blob branch attempt "b"
                    (lambda (finish-target)
                      (before-delivery)
                      (list (finish-source) (finish-target))))))
         ([source-bytes target-bytes]
          ;; Both Git children have finished. Revalidate before parsing,
          ;; returning any payload, or letting the caller mutate a cache.
          (and (current?)
               (let ((source (decode-evidence index source-bytes))
                     (target (decode-evidence branch target-bytes)))
                 (and (related? source target) (list source target))))))))

(def (trace-count attempt side kind)
  (let (path (trace-path attempt side kind))
    (if (file-exists? path)
      (length (call-with-input-file path read-all-as-lines))
      0)))

(def (check-refused-read context index branch source-grant target-grant prepared attempt)
  (check (read-protected-pair context index branch source-grant target-grant prepared attempt) => #f)
  (for-each (lambda (side)
              (check (trace-count attempt side "process") => 0)
              (check (trace-count attempt side "pack") => 0))
            '("a" "b"))
  (displayln "GIT-PREOPEN-DENIED " attempt " process-events=0 pack-accesses=0"))

(def (check-read-started attempt)
  (for-each
   (lambda (side)
     (check (> (trace-count attempt side "pack") 0) => #t)
     (let (trace (call-with-input-file (trace-path attempt side "process") read-all-as-string))
       (check (and (string-contains trace "\"event\":\"start\"") #t) => #t)
       (check (string-contains trace "\"event\":\"exit\"") => #f)))
   '("a" "b")))

(def (select! context index branch source-grant target-grant)
  (check (related? index branch) => #t)
  (let* ((source-id-value (source-id (evidence-object index)))
         (target-id-value (source-id (evidence-object branch)))
         (target-generation (.ref (evidence-cut branch) 'generation)))
    (context-task-select! (publication-host context) (publication-scope context)
      (.o router-revision: "router-1" search-contract-revision: "search-1"
          org-modules:
          (list (.o source: source-id-value anchor: "related" grant: source-grant query-revision: "query-1"
                    origin: (.o source: target-id-value anchor: "binding" generation: target-generation))
                (.o source: target-id-value anchor: "binding" grant: target-grant query-revision: "query-1"))))))

(def (register! context record)
  (context-query-source! (publication-queries context) (evidence-cut record)))

(def (publish! context ticket query)
  (let ((receipt (agentic-ai-lab-search-factor-observation
                  (string-append (.ref ticket 'search-generation) ":returned")
                  (.ref ticket 'search-generation) ContextTaskOrgEvidenceStage
                  (.ref query 'query-digest) 1 (.ref ticket 'search-source-cut) '() 'derived #t)))
    (context-task-publish! (publication-host context) ticket query receipt)))

(def (check-lineage query record anchor)
  (let ((lineage (.ref query 'lineage)) (object (evidence-object record)))
    (check (.ref lineage 'source) => (source-id object))
    (check (.ref lineage 'generation) => (git-object-generation object))
    (check (.ref lineage 'source-digest) => (git-object-digest object))
    (check (.ref lineage 'contract) => (.ref (evidence-cut record) 'contract-identity))
    (check (.ref lineage 'contract-revision) => "1")
    (check (.ref lineage 'org-ids) => (list anchor))
    (check (.ref (.ref query 'query-receipt) 'admitted?) => #t)))

(def (check-published context index branch)
  (let* ((host (publication-host context))
         (ticket (context-task-start! host))
         (query (context-task-query! host ticket)))
    (check (publish! context ticket query) => 'published)
    (match (.ref (context-task-current host) 'org-queries)
      ([source target]
       (check-lineage source index "related")
       (check-lineage target branch "binding")
       (let (origin (.ref source 'origin))
         (check (.ref origin 'source) => (source-id (evidence-object branch)))
         (check (.ref origin 'anchor) => "binding")
         (check (.ref origin 'generation) => (git-object-generation (evidence-object branch)))
         (check (.ref origin 'query-digest) => (.ref (.ref target 'lineage) 'semantic-digest)))))))

;; Exercise one caller-owned cache sink alongside Lab's real source catalog,
;; query recovery export and publication history.
(def (admit-protected-pair! context cache pair source-grant target-grant)
  (match pair
    ([source target]
     (register! context source)
     (register! context target)
     (select! context source target source-grant target-grant)
     (check-published context source target)
     (for-each
      (lambda (record)
        (hash-put! cache (source-id (evidence-object record)) (evidence-cut record)))
      pair)
     #t)))

(def (accept-protected-read! context cache index branch source-grant target-grant attempt
                            (before-delivery void))
  (let* ((prepared (pair-stamp context index branch source-grant target-grant))
         (pair (read-protected-pair context index branch source-grant target-grant
                                    prepared attempt before-delivery)))
    (and pair (admit-protected-pair! context cache pair source-grant target-grant))))

(def (check-cached-cut context cache object)
  (let* ((key (source-id object))
         (cached (hash-get cache key))
         (source (context-query-current-source (publication-queries context) key)))
    (check (.ref cached 'generation) => (git-object-generation object))
    (check (.ref cached 'source-digest) => (git-object-digest object))
    (check (.ref source 'semantic-digest) => (.ref cached 'semantic-digest))))

(def (check-revoked-inflight side)
  (let* ((old-index (read-object-metadata "a1-index" "index.org"))
         (old-branch (read-object-metadata "b1-branch" BranchPath))
         (index (read-object-metadata "a2-index" "index.org"))
         (branch (read-object-metadata "b2-branch" BranchPath))
         (context (make-context old-index))
         (queries (publication-queries context))
         (host (publication-host context))
         (cache (make-hash-table))
         (attempt (string-append "inflight-revoked-" side)))
    (grant! context "source" index "related")
    (grant! context "target" branch "binding")
    ;; Reuse the producer-verified, committed generation-one snapshots to
    ;; warm the sinks. Generation two always enters through the Git batch port.
    (check (and (pair-stamp context old-index old-branch "source" "target")
                (admit-protected-pair! context cache
                  (list (load-evidence "a1-index" "index.org")
                        (load-evidence "b1-branch" BranchPath))
                  "source" "target"))
           => #t)
    (check-cached-cut context cache old-index)
    (check-cached-cut context cache old-branch)
    (let* ((ledger (context-query-export queries))
           (task-history (length (context-task-history host))))
      (check (accept-protected-read! context cache index branch "source" "target" attempt
               (lambda ()
                 (check-read-started attempt)
                 (displayln "GIT-INFLIGHT-STARTED " side " both-payloads-started")
                 (revoke! context side)
                 (displayln "GIT-INFLIGHT-REVOKED " side)))
             => #f)
      (check (context-task-current host) => #f)
      (check (length (context-task-history host)) => task-history)
      (let (after (context-query-export queries))
        (check (map (lambda (cut) (.ref cut 'semantic-digest)) (.ref after 'history)) =>
               (map (lambda (cut) (.ref cut 'semantic-digest)) (.ref ledger 'history)))
        (check (map (lambda (query) (.ref query 'semantic-digest)) (.ref after 'queries)) =>
               (map (lambda (query) (.ref query 'semantic-digest)) (.ref ledger 'queries))))
      (check (hash-length cache) => 2)
      (check-cached-cut context cache old-index)
      (check-cached-cut context cache old-branch)
      (displayln "GIT-INFLIGHT-DISCARDED " side " result=absent cache-generation=1"))
    ;; Fresh authorization has to issue a new physical read before generation
    ;; two is visible. The denied bytes must have left neither replay state nor
    ;; a generation-two cache entry.
    (let (restored (string-append side "-restored"))
      (if (equal? side "source")
        (grant! context restored index "related")
        (grant! context restored branch "binding"))
      (check (accept-protected-read! context cache index branch
               (if (equal? side "source") restored "source")
               (if (equal? side "target") restored "target")
               (string-append attempt "-fresh")) => #t)
      (check-cached-cut context cache index)
      (check-cached-cut context cache branch))))

(def gitops-cross-repo-test
  (test-suite "PR 34 GitOps repository bindings and Lab publication"
    (yuanxu-test-case "Lab's existing base and WorkTrunk Org Contracts"
      (for-each
       (lambda (name)
         (let* ((template-directory
                 (string-append "modules/ai-agentic-context/org-contract/templates/" name))
                (profile (.o (:: @ OrgDirectoryProfile.) directory: template-directory))
                (result (org-directory-validate profile)))
           (check (.ref result 'complete?) => #t)
           (check (.ref result 'passed?) => #t)))
       '("gitops-worktree" "worktree"))
      (displayln "LAB-TEMPLATE-CONTRACTS-OK"))
    (yuanxu-test-case "Git properties, distinct Contracts, and explicit origin"
      (let* ((index (load-evidence "a2-index" "index.org"))
             (worktree (load-evidence "a2-worktree" "topology.org"))
             (local-branch (load-evidence "a2-branch" BranchPath))
             (branch (load-evidence "b2-branch" BranchPath))
             (alias (load-evidence "b-alias-branch" BranchPath)))
        (check (bound? index IndexContract) => #t)
        (check (bound? (load-evidence "b2-index" "index.org") IndexContract) => #t)
        (check (bound? worktree WorktreeContract) => #t)
        (check (bound? (load-evidence "b2-worktree" "topology.org") WorktreeContract) => #t)
        (check (bound? local-branch BranchContract) => #t)
        (check (bound? branch BranchContract) => #t)
        (check (bound? worktree BranchContract) => #f)
        (check (bound? branch WorktreeContract) => #f)
        (check (related? index branch) => #t)
        (check (git-object-ref (evidence-object local-branch)) => (git-object-ref (evidence-object branch)))
        (check (property (headline local-branch "binding") "CUSTOM_ID") =>
               (property (headline branch "binding") "CUSTOM_ID"))
        (check (related? index local-branch) => #f)
        (check (source-id (evidence-object alias)) => (source-id (evidence-object branch)))
        (check (git-object-oid (evidence-object alias)) => (git-object-oid (evidence-object branch)))
        (check (bound? alias BranchContract) => #f)
        (check (related? index alias) => #f)
        (check (related? index (load-evidence "b1-branch" BranchPath)) => #f)
        (displayln "GIT-BINDINGS-OK")))
    (yuanxu-test-case "Both grants and exact source generations are required"
      (let* ((index (load-evidence "a1-index" "index.org"))
             (branch (load-evidence "b1-branch" BranchPath))
             (updated (load-evidence "b2-branch" BranchPath))
             (new-index (load-evidence "a2-index" "index.org"))
             (context (make-context (evidence-object index)))
             (host (publication-host context)))
        (register! context index)
        (register! context branch)
        (select! context index branch "source" "target")
        (grant! context "target" (evidence-object branch) "binding")
        (check (context-task-start! host) => #f)
        (grant! context "source" (evidence-object index) "related")
        (check-published context index branch)
        (select! context index branch "source" "absent-target")
        (check (context-task-start! host) => #f)
        (select! context index branch "source" "target")
        (check-published context index branch)
        (let* ((ticket (context-task-start! host))
               (query (context-task-query! host ticket)))
          (revoke! context "target")
          (check (context-task-current host) => #f)
          (check (context-task-start! host) => #f)
          (check (eq? (publish! context ticket query) 'published) => #f))
        (grant! context "target-restored" (evidence-object branch) "binding")
        (select! context index branch "source" "target-restored")
        (check-published context index branch)
        (register! context updated)
        (check (context-task-current host) => #f)
        (check (context-task-start! host) => #f)
        (register! context new-index)
        (grant! context "source-updated" (evidence-object new-index) "related")
        (grant! context "target-updated" (evidence-object updated) "binding")
        (select! context new-index updated "source-updated" "target-updated")
        (check-published context new-index updated)
        (revoke! context "source-updated")
        (check (context-task-current host) => #f)
        (check (context-task-start! host) => #f)
        (displayln "ORG-LINEAGE-AND-GRANTS-OK")))
    (yuanxu-test-case "Both current grants precede the first Git object read"
      (let* ((index (read-object-metadata "a2-index" "index.org"))
             (branch (read-object-metadata "b2-branch" BranchPath))
             (context (make-context index)))
        (grant! context "target" branch "binding")
        (let (prepared (pair-stamp context index branch "source" "target"))
          (check prepared => #f)
          (check-refused-read context index branch "source" "target" prepared "missing-source"))
        (grant! context "source" index "related")
        (let (prepared (pair-stamp context index branch "source" "missing-target"))
          (check prepared => #f)
          (check-refused-read context index branch "source" "missing-target" prepared "missing-target"))
        (let (prepared (pair-stamp context index branch "source" "target"))
          (check (and prepared #t) => #t)
          (revoke! context "source")
          (check-refused-read context index branch "source" "target" prepared "revoked-source"))
        (grant! context "source-restored" index "related")
        (let (prepared (pair-stamp context index branch "source-restored" "target"))
          (check (and prepared #t) => #t)
          (revoke! context "target")
          (check-refused-read context index branch "source-restored" "target" prepared "revoked-target"))
        (grant! context "target-restored" branch "binding")
        (let* ((prepared (pair-stamp context index branch "source-restored" "target-restored"))
               (pair (read-protected-pair context index branch "source-restored" "target-restored" prepared "allowed")))
          (check (and pair #t) => #t)
          (for-each (lambda (side)
                      (let ((process-events (trace-count "allowed" side "process"))
                            (pack-accesses (trace-count "allowed" side "pack")))
                        (check (> process-events 0) => #t)
                        (check (> pack-accesses 0) => #t)
                        (displayln "GIT-PREOPEN-ALLOWED " side " process-events=" process-events
                                   " pack-accesses=" pack-accesses)))
                    '("a" "b"))
          (match pair
            ([source target]
             (register! context source)
             (register! context target)
             (select! context source target "source-restored" "target-restored")
             (check-published context source target))))
        (displayln "GIT-PREOPEN-AUTHORITY-OK")))))

(def gitops-inflight-test
  (test-suite "PR 34 Git read delivery"
    (yuanxu-test-case "Revocation after Git starts discards both payloads before admission"
      (for-each check-revoked-inflight '("source" "target"))
      (displayln "GIT-INFLIGHT-AUTHORITY-OK"))))
