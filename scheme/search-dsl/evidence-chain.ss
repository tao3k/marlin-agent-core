;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Scenario B Evidence-Chain Accumulation, Stratified ASCENT
;;; Truth Maintenance, Doyle/ATMS Multi-Justification Retraction, Bi-Temporal
;;; Causality & KV-Cache Epoch Fencing (`scheme/search-dsl/evidence-chain.ss`).
;;;
;;; Refines upstream `deterministic-engineering-qnt` formal models directly inside
;;; Marlin's Search DSL:
;;;   - `engineering.defense.search-evidence-chain-integrity`
;;;   - `engineering.defense.durable-memory-tms` (B8: M2->M1, ATMS, poison tombstones)
;;;   - `engineering.defense.bounded-negative-revalidation` (DE01 negative-domain expansion)
;;;   - `engineering.defense.epistemic-state-lattice` (Anti-LWW conflict preservation)
;;;   - `engineering.defense.context-distillation-provenance` (KV-cache pins & integrity meet)
(import (only-in :clan/poo/object .cc .get .o .ref .slot?)
        (only-in :std/crypto/digest sha256)
        (only-in :std/encoding/hex hex-encode)
        (only-in :std/list/list append-map delete-duplicates/hash filter filter-map)
        (only-in :gerbil-ascent/rules/func define-program)
        (only-in :gerbil-ascent/rules/declaration rule-module-program)
        (only-in :gerbil-ascent/program/api
                 gerbil-ascent-open-session
                 gerbil-ascent-session-replace-source!
                 gerbil-ascent-session-run)
        (only-in "../policy/governance.ss"
                 make-governance-evidence-revision
                 evaluate-bitemporal-truth-maintenance
                 make-temporal-governance-event
                 evaluate-temporal-causal-governance
                 verify-temporal-causal-governance)
        "./objects.ss"
        "./funcs.ss")
(export compute-edb-file-fingerprints
        compute-scope-members
        extract-evidence-path-layer
        make-search-evidence-store
        make-search-evidence-claim
        admit-search-evidence-claim
        harvest-choice-result-evidence
        harvest-ascent-result-evidence
        harvest-dusa-scenario-b-evidence
        SearchEvidenceTruthMaintenanceProgram
        reconcile-search-evidence-chain
        materialize-evidence-chain-edb
        evaluate-search-dsl-with-evidence
        evaluate-session-search-dsl-step
        verify-evidence-causal-trajectory)

;; ============================================================================
;; 1. Canonical Hashing & Dual File Fingerprinting (Content vs Structural AST)
;; ============================================================================

(def (evidence-sha256 value)
  (string-append
    "sha256:"
    (hex-encode
      (sha256
        (string->utf8
          (call-with-output-string "" (lambda (p) (write value p))))))))

(def (sexp->canonical-string x)
  (call-with-output-string "" (lambda (p) (write x p))))

(def (sort-list xs less?)
  (letrec ((merge
             (lambda (a b)
               (cond
                 ((null? a) b)
                 ((null? b) a)
                 ((less? (car b) (car a)) (cons (car b) (merge a (cdr b))))
                 (else (cons (car a) (merge (cdr a) b))))))
           (msort
             (lambda (lst len)
               (if (<= len 1)
                 lst
                 (let* ((half (quotient len 2))
                        (left (let loop ((i 0) (r lst) (acc '()))
                                (if (= i half) (reverse acc) (loop (+ i 1) (cdr r) (cons (car r) acc)))))
                        (right (list-tail lst half)))
                   (merge (msort left half) (msort right (- len half))))))))
    (msort xs (length xs))))

(def (sort-canonical-sexps xs)
  (sort-list xs (lambda (a b) (string<? (sexp->canonical-string a) (sexp->canonical-string b)))))

(def (edb-rows edb name)
  (let ((pair (assq name edb)))
    (if (and pair (list? (cdr pair))) (cdr pair) '())))

(def (string-prefix-of? prefix s)
  (let ((plen (string-length prefix))
        (slen (string-length s)))
    (and (<= plen slen)
         (string=? (substring s 0 plen) prefix))))

;;; Compute per-file dual fingerprints directly from Open EDB:
;;;   - `structural-digest`: line-number-independent canonical hash of entities,
;;;     edges, and non-line attrs for that file (INVARIANT across comment/whitespace edits!)
;;;   - `content-digest`: uses `content-overrides` alist `((path . hash) ...)` if present,
;;;     otherwise hashes full line-sensitive entities/edges/attrs.
;;; Returns alist: `((path . #(content-digest structural-digest lang)) ...)`
(def (compute-edb-file-fingerprints edb content-overrides: (content-overrides '()))
  (let* ((files (edb-rows edb 'file))
         (entities (edb-rows edb 'entity))
         (edges (edb-rows edb 'edge))
         (attrs (edb-rows edb 'attr))
         (ent-by-fid (make-hash-table))
         (edge-by-path (make-hash-table))
         (attr-by-id (make-hash-table)))
    (for-each
      (lambda (a)
        (when (and (list? a) (>= (length a) 3))
          (let* ((id (list-ref a 0))
                 (k (list-ref a 1))
                 (v (list-ref a 2)))
            (unless (and (string? k)
                         (or (string=? k "verified_tier")
                             (string=? k "verified_resolution")
                             (string=? k "verified_evidence")))
              (hash-put! attr-by-id id (cons (list k v) (or (hash-get attr-by-id id) '())))))))
      attrs)
    (for-each
      (lambda (ent)
        (when (and (list? ent) (>= (length ent) 7))
          (let ((fid (list-ref ent 1)))
            (hash-put! ent-by-fid fid (cons ent (or (hash-get ent-by-fid fid) '()))))))
      entities)
    (for-each
      (lambda (e)
        (when (and (list? e) (>= (length e) 5))
          (let ((p (list-ref e 3)))
            (hash-put! edge-by-path p (cons e (or (hash-get edge-by-path p) '()))))))
      edges)
    (map
      (lambda (f)
        (let* ((fid (list-ref f 0))
               (lang (list-ref f 1))
               (path (list-ref f 2))
               (f-ents (or (hash-get ent-by-fid fid) '()))
               (f-edges (or (hash-get edge-by-path path) '()))
               ;; Structural fingerprint ignores line numbers and doc comments!
               (struct-ents
                 (sort-canonical-sexps
                   (filter-map
                     (lambda (ent)
                       (let ((kind (list-ref ent 3))
                             (lk (list-ref ent 4))
                             (name (list-ref ent 5))
                             (eid (list-ref ent 0)))
                         (and (not (and (string? kind) (string=? kind "doc")))
                              (list kind lk name
                                    (sort-canonical-sexps (or (hash-get attr-by-id eid) '()))))))
                     f-ents)))
               (struct-edges
                 (sort-canonical-sexps
                   (map (lambda (e) (list (list-ref e 0) (list-ref e 1) (list-ref e 2))) f-edges)))
               (struct-digest (evidence-sha256 (list 'structural lang path struct-ents struct-edges)))
               (override (assoc path content-overrides))
               (content-digest
                 (if override
                   (cdr override)
                   (evidence-sha256
                     (list 'content lang path
                           (sort-canonical-sexps f-ents)
                           (sort-canonical-sexps f-edges))))))
          (cons path (vector content-digest struct-digest lang))))
      files)))

;;; Compute canonical sorted member keys inside a watched negative-scope prefix
;;; (`"scope:<path-prefix>"`) for DE01 bounded negative-domain expansion checks.
(def (compute-scope-members edb scope-domain)
  (let* ((prefix (if (string-prefix-of? "scope:" scope-domain)
                   (substring scope-domain 6 (string-length scope-domain))
                   scope-domain))
         (files (filter (lambda (f) (string-prefix-of? prefix (list-ref f 2)))
                        (edb-rows edb 'file)))
         (fids (let ((tbl (make-hash-table)))
                 (for-each (lambda (f) (hash-put! tbl (list-ref f 0) (list-ref f 2))) files)
                 tbl))
         (file-keys (map (lambda (f) (string-append "file:" (list-ref f 2))) files))
         (ent-keys
           (filter-map
             (lambda (ent)
               (let ((p (hash-get fids (list-ref ent 1))))
                 (and p
                      (not (and (string? (list-ref ent 3)) (string=? (list-ref ent 3) "doc")))
                      (string-append "sym:" p ":" (list-ref ent 3) ":" (list-ref ent 5)))))
             (edb-rows edb 'entity))))
    (sort-list (delete-duplicates/hash (append file-keys ent-keys)) string<?)))

;; ============================================================================
;; 2. Pure Immutable Evidence Store & M2 -> M1 Promotion Gate (B8 / TMS)
;; ============================================================================

(def (make-search-evidence-store edb
                                 epoch: (epoch 1)
                                 generation: (generation #f)
                                 content-overrides: (content-overrides '())
                                 watched-scopes: (watched-scopes '())
                                 frozen-cuts: (frozen-cuts '()))
  (let ((fps (compute-edb-file-fingerprints edb content-overrides: content-overrides))
        (scopes (map (lambda (d) (cons d (compute-scope-members edb d))) watched-scopes)))
    (.o bind-these: #f
        kind: 'marlin.search-evidence-store.v1
        epoch: epoch
        generation: generation
        claims: '()
        tombstones: '()
        revoked-principals: '()
        retracted-ids: '()
        frozen-cuts: frozen-cuts
        file-fingerprints: fps
        scope-snapshots: scopes
        journal-revisions: '()
        last-rev-by-claim: '()
        next-rev-seq: 1)))

(def (make-search-evidence-claim claim-id kind target-key value
                                 tier: (tier 'M2-provisional)
                                 file-deps: (file-deps '())
                                 content-sensitive?: (content-sensitive? #f)
                                 neg-domains: (neg-domains '())
                                 justifications: (justifications '())
                                 oracle-receipt: (oracle-receipt #f)
                                 attesters: (attesters '())
                                 proposer: (proposer "agent")
                                 source-integrity: (source-integrity 'internal-agent)
                                 epoch: (epoch 1))
  (let ((digest (evidence-sha256 (list 'search-claim kind target-key value))))
    (.o bind-these: #f
        kind: 'marlin.search-evidence-claim.v1
        claim-id: claim-id
        tier: tier
        claim-kind: kind
        target-key: target-key
        value: value
        file-deps: file-deps
        content-sensitive?: content-sensitive?
        neg-domains: neg-domains
        justifications: justifications
        oracle-receipt: oracle-receipt
        attesters: attesters
        proposer: proposer
        source-integrity: source-integrity
        epoch: epoch
        claim-digest: digest)))

(def (independent-valid-attesters claim revoked-principals)
  (let ((proposer (.ref claim 'proposer)))
    (delete-duplicates/hash
      (filter (lambda (att)
                (and (string? att)
                     (not (string=? att proposer))
                     (not (member att revoked-principals))))
              (.ref claim 'attesters)))))

;;; Admit a candidate search evidence claim under B8 / SearchEvidenceChainIntegrity:
;;;   1. Untrusted external inputs are quarantined in M2 (never promoted to M1).
;;;   2. Any claim matching a pinned poison tombstone is REJECTED (anti-repoisoning).
;;;   3. Any claim proposed by a revoked principal is REJECTED.
;;;   4. Promotion to M1 requires EITHER a verified oracle-receipt OR >= quorum-k
;;;      independent non-revoked attesters (attester != proposer).
(def (admit-search-evidence-claim store claim
                                  edb: (edb #f)
                                  quorum-k: (quorum-k 2))
  (let* ((tombstones (.ref store 'tombstones))
         (revoked (.ref store 'revoked-principals))
         (digest (.ref claim 'claim-digest))
         (proposer (.ref claim 'proposer))
         (src-int (.ref claim 'source-integrity))
         (oracle (.ref claim 'oracle-receipt))
         (att-count (length (independent-valid-attesters claim revoked))))
    (cond
      ((member digest tombstones)
       (.o bind-these: #f
           admitted?: #f
           status: 'rejected
           reason: 'poison-tombstone-blocked
           store: store))
      ((member proposer revoked)
       (.o bind-these: #f
           admitted?: #f
           status: 'rejected
           reason: 'revoked-principal-blocked
           store: store))
      ((eq? src-int 'untrusted-external)
       (let* ((quarantined (.cc claim 'tier 'M2-provisional 'epoch (.ref store 'epoch)))
              (next-claims (cons (cons (.ref claim 'claim-id) quarantined) (.ref store 'claims))))
         (.o bind-these: #f
             admitted?: #f
             status: 'quarantined-m2
             reason: 'untrusted-external-source
             store: (.cc store 'claims next-claims))))
      ((not (or (and (string? oracle) (> (string-length oracle) 0))
                (>= att-count quorum-k)))
       (let* ((quarantined (.cc claim 'tier 'M2-provisional 'epoch (.ref store 'epoch)))
              (next-claims (cons (cons (.ref claim 'claim-id) quarantined) (.ref store 'claims))))
         (.o bind-these: #f
             admitted?: #f
             status: 'quarantined-m2
             reason: 'missing-oracle-or-quorum
             store: (.cc store 'claims next-claims))))
      (else
       (let* ((promoted (.cc claim 'tier 'M1-verified 'epoch (.ref store 'epoch)))
              (cid (.ref promoted 'claim-id))
              (seq (.ref store 'next-rev-seq))
              (rev-id (string-append "rev-assert-" (number->string seq)))
              (rev (make-governance-evidence-revision
                     rev-id
                     cid
                     'assert
                     #f
                     seq
                     valid-start: 0
                     valid-end: 10000
                     content-id: digest))
              (new-scopes
                (if (and edb (pair? (.ref promoted 'neg-domains)))
                  (let loop ((ds (.ref promoted 'neg-domains))
                             (acc (.ref store 'scope-snapshots)))
                    (if (null? ds)
                      acc
                      (let ((d (car ds)))
                        (loop (cdr ds)
                              (if (assoc d acc)
                                acc
                                (cons (cons d (compute-scope-members edb d)) acc))))))
                  (.ref store 'scope-snapshots)))
              (next-store
                (.cc store
                     'claims (cons (cons cid promoted)
                                   (filter (lambda (p) (not (string=? (car p) cid)))
                                           (.ref store 'claims)))
                     'scope-snapshots new-scopes
                     'journal-revisions (append (.ref store 'journal-revisions) (list rev))
                     'last-rev-by-claim (cons (cons cid rev-id)
                                              (filter (lambda (p) (not (string=? (car p) cid)))
                                                      (.ref store 'last-rev-by-claim)))
                     'next-rev-seq (+ seq 1))))
         (.o bind-these: #f
             admitted?: #t
             status: 'promoted-m1
             reason: 'verified
             claim: promoted
             store: next-store))))))

;; ============================================================================
;; 3. Scenario B Dusa Witness Harvesting ("越搜越强" — Oracle-Backed M1 Facts)
;; ============================================================================

(def (choice-atom->string x)
  (cond
    ((string? x) x)
    ((symbol? x) (symbol->string x))
    ((number? x) (number->string x))
    (else (sexp->canonical-string x))))

;;; Pure helper: project a workspace-relative path to its architectural layer
;;; (`crates/<crate>`, `crates/<crate>/tests`, `scheme/<subpkg>`, or top dir).
(def (string-split-slash s)
  (let ((len (string-length s)))
    (let loop ((i 0) (start 0) (acc '()))
      (cond
        ((= i len)
         (reverse (if (> i start) (cons (substring s start i) acc) acc)))
        ((char=? (string-ref s i) #\/)
         (loop (+ i 1) (+ i 1)
               (if (> i start) (cons (substring s start i) acc) acc)))
        (else (loop (+ i 1) start acc))))))

(def (extract-evidence-path-layer path)
  (if (not (string? path))
    #f
    (let ((segs (string-split-slash path)))
      (cond
        ((null? segs) #f)
        ((and (string=? (car segs) "crates") (>= (length segs) 3)
              (or (string=? (caddr segs) "tests") (string=? (caddr segs) "benches")))
         (string-append "crates/" (cadr segs) "/tests"))
        ((and (string=? (car segs) "crates") (>= (length segs) 2))
         (string-append "crates/" (cadr segs)))
        ((and (string=? (car segs) "scheme") (>= (length segs) 2))
         (string-append "scheme/" (cadr segs)))
        (else (car segs))))))

;;; Harvest invariant `[deduction]` / single-world agreements directly from an
;;; already-evaluated Dusa `search-result` vector (avoiding double evaluation).
;;; Multi-world `[branch]` keys are preserved explicitly in `ambiguous` (Anti-LWW).
(def (harvest-choice-result-evidence store res edb
                                     proposer: (proposer "dusa-oracle")
                                     claim-prefix: (claim-prefix "dusa")
                                     neg-domains: (neg-domains '())
                                     strict?: (strict? #f))
  (let* ((mode (and (vector? res) (>= (vector-length res) 4) (vector-ref res 1)))
         (target-rel (and (vector? res) (>= (vector-length res) 4) (vector-ref res 2)))
         (payload (and (vector? res) (>= (vector-length res) 4) (vector-ref res 3))))
    (cond
      ((and (eq? mode 'choice) (vector? payload) (eq? (vector-ref payload 0) 'choice-report))
       (let* ((world-count (vector-ref payload 1))
              (worlds (vector-ref payload 4))
              (vals-by-key (make-hash-table))
              (prov-by-key (make-hash-table))
              (all-file-paths (map (lambda (f) (list-ref f 2)) (edb-rows edb 'file))))
         (for-each
           (lambda (w)
             (let ((bindings (vector-ref w 2))
                   (witnesses (vector-ref w 3)))
               (for-each
                 (lambda (b)
                   (when (and (list? b) (>= (length b) 2))
                     (let* ((k (choice-atom->string (car b)))
                            (v (choice-atom->string (cadr b)))
                            (prev (or (hash-get vals-by-key k) '())))
                       (unless (member v prev)
                         (hash-put! vals-by-key k (cons v prev))))))
                 bindings)
               (for-each
                 (lambda (wt)
                   (let ((wkind (vector-ref wt 0))
                         (wfact (vector-ref wt 1)))
                     (when (and (list? wfact) (>= (length wfact) 3))
                       (let ((k (choice-atom->string (cadr wfact))))
                         (hash-put! prov-by-key k wkind)))))
                 witnesses)))
           worlds)
         (let* ((claim-kind (cond
                              ((eq? target-rel 'tier) 'dusa-tier)
                              ((eq? target-rel 'resolve) 'dusa-resolution)
                              (else 'ascent-fact)))
                (sorted-keys (sort-list (hash-keys vals-by-key) string<?)))
           (let loop ((ks sorted-keys)
                      (cur-store store)
                      (harvested '())
                      (ambiguous '()))
             (if (null? ks)
               (.o bind-these: #f
                   store: cur-store
                   world-count: world-count
                   harvested: (reverse harvested)
                   ambiguous: (reverse ambiguous))
               (let* ((k (car ks))
                      (vs (sort-list (hash-get vals-by-key k) string<?))
                      (prov (or (hash-get prov-by-key k) 'derived)))
                 (if (> (length vs) 1)
                   ;; Multi-world branching ambiguity: NEVER collapse via LWW!
                   (loop (cdr ks) cur-store harvested (cons (list k vs prov) ambiguous))
                   (let* ((v (car vs))
                          (cid (string-append claim-prefix ":" (symbol->string target-rel) ":" k))
                          (dep-files
                            (cond
                              ((member k all-file-paths) (list k))
                              (else
                               (let ((matched
                                       (filter (lambda (p)
                                                 (or (string-prefix-of? p k)
                                                     (string-prefix-of? p v)))
                                               all-file-paths)))
                                 (if (pair? matched) matched all-file-paths)))))
                          (receipt (evidence-sha256
                                     (list 'dusa-oracle-witness target-rel k v prov world-count dep-files)))
                          (claim (make-search-evidence-claim
                                   cid claim-kind k v
                                   tier: 'M1-verified
                                   file-deps: dep-files
                                   content-sensitive?: #f
                                   neg-domains: neg-domains
                                   oracle-receipt: receipt
                                   proposer: proposer
                                   source-integrity: 'trusted-oracle
                                   epoch: (.ref cur-store 'epoch)))
                          (adm (admit-search-evidence-claim cur-store claim edb: edb)))
                     (loop (cdr ks)
                           (.ref adm 'store)
                           (if (.ref adm 'admitted?) (cons cid harvested) harvested)
                           ambiguous)))))))))
      (strict?
       (error "harvest-dusa-scenario-b-evidence expected SAT choice-report" payload))
      (else
       (.o bind-these: #f
           store: store
           world-count: 0
           harvested: '()
           ambiguous: '())))))

;;; Run a REAL Scenario B Dusa query (`resolve` disambiguation or `tier` placement)
;;; over `edb`, harvest every invariant `[deduction]` or single-world agreement into
;;; oracle-backed M1 evidence claims (preserving multi-world `[branch]` ambiguity
;;; without LWW collapse!), and return `(.o store: ... harvested: ... ambiguous: ... worlds: ...)`.
(def (harvest-dusa-scenario-b-evidence store dusa-query edb
                                       registry: (registry SearchLanguageRegistryValue)
                                       proposer: (proposer "dusa-oracle")
                                       claim-prefix: (claim-prefix "dusa")
                                       neg-domains: (neg-domains '()))
  (let ((res (evaluate-search-dsl-over-edb dusa-query edb registry)))
    (harvest-choice-result-evidence
      store res edb
      proposer: proposer
      claim-prefix: claim-prefix
      neg-domains: neg-domains
      strict?: #t)))

(def (cell-looks-like-path? s known-path-tbl)
  (and (string? s)
       (> (string-length s) 2)
       (or (hash-get known-path-tbl s)
           (and (string-contains-char? s #\/)
                (string-contains-char? s #\.)))))

(def (string-contains-char? s ch)
  (let ((len (string-length s)))
    (let loop ((i 0))
      (cond
        ((>= i len) #f)
        ((char=? (string-ref s i) ch) #t)
        (else (loop (+ i 1)))))))

(def (string-contains-sub? hay needle)
  (let ((hlen (string-length hay))
        (nlen (string-length needle)))
    (cond
      ((= nlen 0) #t)
      ((> nlen hlen) #f)
      (else
       (let loop ((i 0))
         (cond
           ((> (+ i nlen) hlen) #f)
           ((string=? (substring hay i (+ i nlen)) needle) #t)
           (else (loop (+ i 1)))))))))

;;; Harvest verified structural M1 facts directly from an L1 `:gerbil-ascent`
;;; `search-result` vector (bounded to `max-claims` high-signal anchors so
;;; multi-turn sessions accumulate crisp, deduplicated M1 evidence).
(def (harvest-ascent-result-evidence store res edb
                                     proposer: (proposer "ascent-oracle")
                                     claim-prefix: (claim-prefix "ascent")
                                     max-claims: (max-claims 24))
  (let* ((mode (and (vector? res) (>= (vector-length res) 4) (vector-ref res 1)))
         (target-rel (and (vector? res) (>= (vector-length res) 4) (vector-ref res 2)))
         (rows (and (vector? res) (>= (vector-length res) 4) (vector-ref res 3))))
    (if (not (and (eq? mode 'ascent) (list? rows) (pair? rows)))
      (.o bind-these: #f store: store harvested: '() seen-paths: '())
      (let ((known-paths (make-hash-table))
            (existing-cids (make-hash-table)))
        (for-each (lambda (f) (when (>= (length f) 3) (hash-put! known-paths (list-ref f 2) #t)))
                  (edb-rows edb 'file))
        (for-each (lambda (pair) (hash-put! existing-cids (car pair) #t))
                  (.ref store 'claims))
        (let loop ((rest rows)
                   (cur-store store)
                   (harvested '())
                   (seen-paths '())
                   (count 0))
          (if (or (null? rest) (>= count max-claims))
            (.o bind-these: #f
                store: cur-store
                harvested: (reverse harvested)
                seen-paths: (delete-duplicates/hash seen-paths))
            (let* ((row (car rest))
                   (cells (if (list? row) row (list row)))
                   (paths (filter (lambda (c) (cell-looks-like-path? c known-paths)) cells))
                   (primary (if (pair? cells) (choice-atom->string (car cells)) ""))
                   (first-path (and (pair? paths) (car paths))))
              (if (or (not first-path) (= (string-length primary) 0))
                (loop (cdr rest) cur-store harvested (append paths seen-paths) count)
                (let* ((cid (string-append claim-prefix ":" (symbol->string target-rel) ":" primary "@" first-path))
                       (already? (hash-get existing-cids cid)))
                  (if already?
                    (loop (cdr rest) cur-store harvested (append paths seen-paths) count)
                    (let* ((tkey (string-append (symbol->string target-rel) ":" primary))
                           (val (string-append first-path "::" (choice-atom->string target-rel)))
                           (receipt (evidence-sha256 (list 'ascent-oracle-row target-rel primary paths)))
                           (claim (make-search-evidence-claim
                                    cid 'ascent-fact tkey val
                                    tier: 'M1-verified
                                    file-deps: (delete-duplicates/hash paths)
                                    content-sensitive?: #f
                                    oracle-receipt: receipt
                                    proposer: proposer
                                    source-integrity: 'trusted-oracle
                                    epoch: (.ref cur-store 'epoch)))
                           (adm (admit-search-evidence-claim cur-store claim edb: edb)))
                      (hash-put! existing-cids cid #t)
                      (loop (cdr rest)
                            (.ref adm 'store)
                            (if (.ref adm 'admitted?) (cons cid harvested) harvested)
                            (append paths seen-paths)
                            (+ count 1)))))))))))))

;; ============================================================================
;; 4. Stratified ASCENT Evidence-Chain Truth Maintenance & KV-Cache Fencing
;; ============================================================================

(define-program
  (make-search-evidence-truth-maintenance-program)
  (sources
    claim
    claim_file_dep
    file_content_drifted
    file_structural_drifted
    claim_neg_domain
    neg_domain_expanded
    explicit_retracted
    revoked_principal
    atms_surviving
    frozen_cut_item
    kv_cache_entry
    active_epoch)
  (outputs
    claim_invalidated
    claim_reusable
    epistemic_conflict
    frozen_cut_preserved
    kv_entry_rejected
    kv_entry_admitted)
  (relation claim
    ((cid string?) (tier string?) (ckind string?) (target_key string?)
     (val string?) (proposer string?) (epoch string?) (has_oracle string?)))
  (relation claim_file_dep ((cid string?) (path string?) (content_sensitive string?)))
  (relation file_content_drifted ((path string?)))
  (relation file_structural_drifted ((path string?)))
  (relation claim_neg_domain ((cid string?) (domain string?)))
  (relation neg_domain_expanded ((domain string?) (new_member string?)))
  (relation explicit_retracted ((cid string?)))
  (relation revoked_principal ((proposer string?)))
  (relation atms_surviving ((cid string?)))
  (relation frozen_cut_item ((cut_id string?) (digest string?)))
  (relation kv_cache_entry
    ((kv_id string?) (cid string?) (kv_epoch string?)
     (pinned_retained string?) (untrusted_tainted string?)))
  (relation active_epoch ((ep string?)))

  (relation claim_invalidated ((cid string?) (reason string?)))
  (relation claim_reusable ((cid string?)))
  (relation epistemic_conflict ((target_key string?) (val1 string?) (val2 string?)))
  (relation frozen_cut_preserved ((cut_id string?) (digest string?)))
  (relation kv_entry_rejected ((kv_id string?) (reason string?)))
  (relation kv_entry_admitted ((kv_id string?)))

  ;; 1. Unverified M2 claims cannot act as reusable M1 evidence.
  ((claim_invalidated Cid (lit "m2-unverified-quarantine"))
   <--
   (claim Cid (lit "M2-provisional") _ _ _ _ _ _))

  ;; 2. Structural AST drift invalidates every claim depending on that file.
  ((claim_invalidated Cid (lit "structural-drift"))
   <--
   (claim_file_dep Cid Path _)
   (file_structural_drifted Path))

  ;; 3. Comment-only content drift invalidates ONLY content-sensitive L0 claims
  ;;    (M1 structural AST & Dusa claims with content_sensitive="no" stay REUSABLE!).
  ((claim_invalidated Cid (lit "content-drift-l0"))
   <--
   (claim_file_dep Cid Path (lit "yes"))
   (file_content_drifted Path))

  ;; 4. DE01 Negative-domain expansion invalidates universal/negative claims on that domain.
  ((claim_invalidated Cid (lit "neg-domain-expanded"))
   <--
   (claim_neg_domain Cid Domain)
   (neg_domain_expanded Domain _))

  ;; 5. Explicit poison retraction.
  ((claim_invalidated Cid (lit "explicit-poison-retraction"))
   <--
   (claim Cid _ _ _ _ _ _ _)
   (explicit_retracted Cid))

  ;; 6. B3 -> B8 Compromised principal cascade (preserves independent oracle-backed claims).
  ((claim_invalidated Cid (lit "revoked-principal-unverified"))
   <--
   (claim Cid _ _ _ _ Prop _ (lit "no"))
   (revoked_principal Prop))

  ;; 7. ATMS multi-justification loss (derived claim lost ALL conjunctive support sets).
  ((claim_invalidated Cid (lit "atms-all-justifications-lost"))
   <--
   (claim Cid (lit "M1-verified") _ _ _ _ _ _)
   (not (atms_surviving Cid)))

  ;; 8. Reusable frontier: strictly M1 claims with zero invalidation reasons.
  ((claim_reusable Cid)
   <--
   (claim Cid (lit "M1-verified") _ _ _ _ _ _)
   (not (claim_invalidated Cid _)))

  ;; 9. Anti-LWW Epistemic Conflict Preservation: two surviving claims with distinct values
  ;;    for the same target key surface explicitly as Conflict!
  ((epistemic_conflict TKey V1 V2)
   <--
   (claim C1 (lit "M1-verified") _ TKey V1 _ _ _)
   (claim C2 (lit "M1-verified") _ TKey V2 _ _ _)
   (claim_reusable C1)
   (claim_reusable C2)
   (if (V1 V2) (string<? V1 V2)))

  ;; 10. Frozen historical cuts are unconditionally preserved (anti-P3 global purge).
  ((frozen_cut_preserved Cut Dig) <-- (frozen_cut_item Cut Dig))

  ;; 11. KV-Cache & Context Distillation Fencing:
  ;;     Reject stale epoch, zombie/invalidated claim, pinned-constraint amnesia,
  ;;     or untrusted integrity laundering!
  ((kv_entry_rejected KvId (lit "kv-stale-epoch"))
   <--
   (kv_cache_entry KvId _ KvEp _ _)
   (active_epoch CurEp)
   (if (KvEp CurEp) (not (equal? KvEp CurEp))))
  ((kv_entry_rejected KvId (lit "kv-zombie-claim"))
   <--
   (kv_cache_entry KvId Cid _ _ _)
   (not (claim_reusable Cid)))
  ((kv_entry_rejected KvId (lit "distillation-pinned-amnesia"))
   <--
   (kv_cache_entry KvId _ _ (lit "no") _))
  ((kv_entry_rejected KvId (lit "integrity-laundering"))
   <--
   (kv_cache_entry KvId _ _ _ (lit "yes")))
  ((kv_entry_admitted KvId)
   <--
   (kv_cache_entry KvId Cid KvEp (lit "yes") (lit "no"))
   (active_epoch KvEp)
   (claim_reusable Cid)
   (not (kv_entry_rejected KvId _)))
  (bounds 4096 4096 4096))

(def SearchEvidenceTruthMaintenanceDeclaration
  (make-search-evidence-truth-maintenance-program))
(def SearchEvidenceTruthMaintenanceProgram
  (rule-module-program SearchEvidenceTruthMaintenanceDeclaration))

;;; Pure functional Knaster-Tarski fixed-point for ATMS multi-justification DAGs:
;;; A claim `c` survives iff:
;;;   - `c` is M1 and NOT directly invalidated, AND
;;;   - EITHER `c` has no explicit justifications (it is a base root fact),
;;;   - OR `c` has at least one justification set `J` in `justifications(c)` where
;;;     EVERY dependency `d` in `J` is in the surviving set!
(def (compute-atms-surviving-ids claims direct-invalid-tbl)
  (let ((surviving (make-hash-table)))
    ;; Seed base M1 root facts (no justifications) that are not directly invalidated.
    (for-each
      (lambda (pair)
        (let* ((cid (car pair))
               (cl (cdr pair)))
          (when (and (eq? (.ref cl 'tier) 'M1-verified)
                     (not (hash-get direct-invalid-tbl cid))
                     (null? (.ref cl 'justifications)))
            (hash-put! surviving cid #t))))
      claims)
    ;; Monotonic fixed-point propagation across derived multi-justification claims.
    (let loop ((changed? #t))
      (when changed?
        (let ((next-changed? #f))
          (for-each
            (lambda (pair)
              (let* ((cid (car pair))
                     (cl (cdr pair))
                     (justs (.ref cl 'justifications)))
                (when (and (eq? (.ref cl 'tier) 'M1-verified)
                           (not (hash-get surviving cid))
                           (not (hash-get direct-invalid-tbl cid))
                           (pair? justs))
                  (when (ormap (lambda (jset)
                                 (and (pair? jset)
                                      (andmap (lambda (dep) (and (hash-get surviving dep) #t))
                                              jset)))
                               justs)
                    (hash-put! surviving cid #t)
                    (set! next-changed? #t)))))
            claims)
          (loop next-changed?))))
    surviving))

(def (run-evidence-tm-ascent sources-alist)
  (let ((session (gerbil-ascent-open-session SearchEvidenceTruthMaintenanceProgram)))
    (gerbil-ascent-session-run session)
    (for-each
      (lambda (entry)
        (gerbil-ascent-session-replace-source! session (car entry) (cdr entry)))
      sources-alist)
    (gerbil-ascent-session-run session)))

(def (rows-of result rel)
  ((.ref result 'rows-of) rel))

;;; Reconcile the accumulated search evidence store against a new EDB snapshot,
;;; optional comment-only content overrides, retroactive poison retractions,
;;; compromised principal revocations, and LLM KV-cache entries.
;;;
;;; Combines:
;;;   1. Dual content vs structural AST drift detection
;;;   2. DE01 negative-domain expansion detection
;;;   3. ATMS multi-justification fixed-point survival
;;;   4. Stratified `:gerbil-ascent` `SearchEvidenceTruthMaintenanceProgram`
;;;   5. Marlin's Bi-Temporal Evidence Journal (`evaluate-bitemporal-truth-maintenance`)
(def (reconcile-search-evidence-chain store next-edb
                                      content-overrides: (content-overrides '())
                                      retract-poison-ids: (retract-poison-ids '())
                                      revoke-principals: (revoke-principals '())
                                      kv-entries: (kv-entries '()))
  (let* ((old-fps (.ref store 'file-fingerprints))
         (new-fps (compute-edb-file-fingerprints next-edb content-overrides: content-overrides))
         (all-paths (delete-duplicates/hash (append (map car old-fps) (map car new-fps))))
         (content-drifted
           (filter
             (lambda (p)
               (let ((o (assoc p old-fps)) (n (assoc p new-fps)))
                 (or (not o) (not n)
                     (not (string=? (vector-ref (cdr o) 0) (vector-ref (cdr n) 0))))))
             all-paths))
         (structural-drifted
           (filter
             (lambda (p)
               (let ((o (assoc p old-fps)) (n (assoc p new-fps)))
                 (or (not o) (not n)
                     (not (string=? (vector-ref (cdr o) 1) (vector-ref (cdr n) 1))))))
             all-paths))
         ;; Detect DE01 negative-domain expansions across watched scopes
         (old-scopes (.ref store 'scope-snapshots))
         (new-scopes (map (lambda (sc) (cons (car sc) (compute-scope-members next-edb (car sc))))
                          old-scopes))
         (neg-expansions
           (append-map
             (lambda (sc)
               (let* ((dom (car sc))
                      (old-ms (cdr sc))
                      (new-ms (cdr (assoc dom new-scopes))))
                 (filter-map
                   (lambda (m) (and (not (member m old-ms)) (list dom m)))
                   new-ms)))
             old-scopes))
         ;; Pin permanent forensic poison tombstones for every retracted poison claim!
         (all-retracted (delete-duplicates/hash (append (.ref store 'retracted-ids) retract-poison-ids)))
         (all-revoked (delete-duplicates/hash (append (.ref store 'revoked-principals) revoke-principals)))
         (claims (.ref store 'claims))
         (new-tombstones
           (delete-duplicates/hash
             (append (.ref store 'tombstones)
                     (filter-map
                       (lambda (cid)
                         (let ((p (assoc cid claims)))
                           (and p (.ref (cdr p) 'claim-digest))))
                       retract-poison-ids))))
         ;; Epoch advances ONLY on structural drift, negative-domain expansion,
         ;; poison retraction, or principal revocation — NEVER on comment-only edits!
         (semantic-change? (or (pair? structural-drifted)
                               (pair? neg-expansions)
                               (pair? retract-poison-ids)
                               (pair? revoke-principals)))
         (next-epoch (if semantic-change? (+ (.ref store 'epoch) 1) (.ref store 'epoch)))
         ;; Compute directly invalidated roots for ATMS fixed-point
         (direct-invalid (make-hash-table)))
    (for-each
      (lambda (pair)
        (let* ((cid (car pair))
               (cl (cdr pair))
               (deps (.ref cl 'file-deps))
               (cs? (.ref cl 'content-sensitive?))
               (ndoms (.ref cl 'neg-domains))
               (prop (.ref cl 'proposer))
               (has-oracle? (and (string? (.ref cl 'oracle-receipt))
                                 (> (string-length (.ref cl 'oracle-receipt)) 0))))
          (when (or (not (eq? (.ref cl 'tier) 'M1-verified))
                    (member cid all-retracted)
                    (and (not has-oracle?) (member prop all-revoked))
                    (ormap (lambda (d) (member d structural-drifted)) deps)
                    (and cs? (ormap (lambda (d) (member d content-drifted)) deps))
                    (ormap (lambda (dom) (assoc dom neg-expansions)) ndoms))
            (hash-put! direct-invalid cid #t))))
      claims)
    (let* ((surviving-tbl (compute-atms-surviving-ids claims direct-invalid))
           (claim-rows
             (map (lambda (pair)
                    (let ((cl (cdr pair)))
                      (list (.ref cl 'claim-id)
                            (symbol->string (.ref cl 'tier))
                            (symbol->string (.ref cl 'claim-kind))
                            (.ref cl 'target-key)
                            (.ref cl 'value)
                            (.ref cl 'proposer)
                            (number->string (.ref cl 'epoch))
                            (if (and (string? (.ref cl 'oracle-receipt))
                                     (> (string-length (.ref cl 'oracle-receipt)) 0))
                              "yes" "no"))))
                  claims))
           (dep-rows
             (append-map
               (lambda (pair)
                 (let* ((cl (cdr pair))
                        (cid (.ref cl 'claim-id))
                        (cs-str (if (.ref cl 'content-sensitive?) "yes" "no")))
                   (map (lambda (d) (list cid d cs-str)) (.ref cl 'file-deps))))
               claims))
           (neg-rows
             (append-map
               (lambda (pair)
                 (let ((cl (cdr pair)))
                   (map (lambda (dom) (list (.ref cl 'claim-id) dom)) (.ref cl 'neg-domains))))
               claims))
           (kv-rows
             (map (lambda (kv)
                    (list (list-ref kv 0)
                          (list-ref kv 1)
                          (number->string (list-ref kv 2))
                          (if (list-ref kv 3) "yes" "no")
                          (if (list-ref kv 4) "yes" "no")))
                  kv-entries))
           (ascent-res
             (run-evidence-tm-ascent
               (list
                 (cons 'claim claim-rows)
                 (cons 'claim_file_dep dep-rows)
                 (cons 'file_content_drifted (map list content-drifted))
                 (cons 'file_structural_drifted (map list structural-drifted))
                 (cons 'claim_neg_domain neg-rows)
                 (cons 'neg_domain_expanded neg-expansions)
                 (cons 'explicit_retracted (map list all-retracted))
                 (cons 'revoked_principal (map list all-revoked))
                 (cons 'atms_surviving (map list (hash-keys surviving-tbl)))
                 (cons 'frozen_cut_item (.ref store 'frozen-cuts))
                 (cons 'kv_cache_entry kv-rows)
                 (cons 'active_epoch (list (list (number->string next-epoch)))))))
           (reusable-ids (sort-list (map car (rows-of ascent-res 'claim_reusable)) string<?))
           (invalidated (rows-of ascent-res 'claim_invalidated))
           (conflicts (rows-of ascent-res 'epistemic_conflict))
           (kv-admitted (sort-list (map car (rows-of ascent-res 'kv_entry_admitted)) string<?))
           (kv-rejected (rows-of ascent-res 'kv_entry_rejected))
           (frozen-preserved (rows-of ascent-res 'frozen_cut_preserved))
           ;; Identify derived claims that survived via ATMS alternative clean justification
           (atms-preserved
             (filter-map
               (lambda (cid)
                 (let ((p (assoc cid claims)))
                   (and p
                        (pair? (.ref (cdr p) 'justifications))
                        (ormap (lambda (jset)
                                 (ormap (lambda (d) (hash-get direct-invalid d)) jset))
                               (.ref (cdr p) 'justifications))
                        cid)))
               reusable-ids))
           ;; Wire Bi-Temporal Truth Maintenance journal revisions (`scheme/policy/governance.ss`)!
           (prev-seq (- (.ref store 'next-rev-seq) 1))
           (last-rev-map (.ref store 'last-rev-by-claim))
           (new-revs+seq
             (let loop ((ids (delete-duplicates/hash (map car invalidated)))
                        (cur-seq (.ref store 'next-rev-seq))
                        (acc '())
                        (rmap last-rev-map))
               (if (null? ids)
                 (vector (reverse acc) cur-seq rmap)
                 (let* ((cid (car ids))
                        (pred-pair (assoc cid rmap))
                        (pred-rev (and pred-pair (cdr pred-pair))))
                   (if pred-rev
                     (let ((retract-id (string-append "rev-retract-" (number->string cur-seq))))
                       (loop (cdr ids)
                             (+ cur-seq 1)
                             (cons (make-governance-evidence-revision
                                     retract-id cid 'retract pred-rev cur-seq)
                                   acc)
                             (filter (lambda (p) (not (string=? (car p) cid))) rmap)))
                     (loop (cdr ids) cur-seq acc rmap))))))
           (appended-revs (vector-ref new-revs+seq 0))
           (final-seq (vector-ref new-revs+seq 1))
           (final-rmap (vector-ref new-revs+seq 2))
           (all-revs (append (.ref store 'journal-revisions) appended-revs))
           (deriv-specs
             (filter-map
               (lambda (pair)
                 (let* ((cid (car pair))
                        (cl (cdr pair))
                        (justs (.ref cl 'justifications)))
                   (and (pair? justs)
                        (list cid
                              "search-evidence-tm"
                              (delete-duplicates/hash (apply append justs))
                              '()))))
               claims))
           (bitemporal-receipt
             (if (and (pair? all-revs) (>= prev-seq 1))
               (evaluate-bitemporal-truth-maintenance
                 "marlin-search-evidence-journal"
                 all-revs
                 deriv-specs
                 prev-seq
                 (max prev-seq (- final-seq 1))
                 100)
               #f))
           ;; Keep only reusable M1 claims + update metadata in next-store
           (next-claims
             (filter (lambda (p) (member (car p) reusable-ids)) claims))
           (next-store
             (.cc store
                  'epoch next-epoch
                  'claims next-claims
                  'tombstones new-tombstones
                  'revoked-principals all-revoked
                  'retracted-ids all-retracted
                  'file-fingerprints new-fps
                  'scope-snapshots new-scopes
                  'journal-revisions all-revs
                  'last-rev-by-claim final-rmap
                  'next-rev-seq final-seq)))
      (.o bind-these: #f
          kind: 'marlin.search-evidence-reconcile-receipt.v1
          previous-epoch: (.ref store 'epoch)
          active-epoch: next-epoch
          content-drifted: content-drifted
          structural-drifted: structural-drifted
          neg-expansions: neg-expansions
          reusable-claims: reusable-ids
          invalidated-claims: invalidated
          atms-preserved-claims: atms-preserved
          conflicts: conflicts
          kv-admitted: kv-admitted
          kv-rejected: kv-rejected
          frozen-cuts-preserved: frozen-preserved
          bitemporal-receipt: bitemporal-receipt
          next-store: next-store))))

;; ============================================================================
;; 5. Compound Search Amplification ("越搜越强") & Bi-Temporal Causal Proof
;; ============================================================================

;;; Materialize ONLY live, admitted, non-conflicted M1 claims from `store` into
;;; Open EDB `attr` rows so subsequent L1 `:gerbil-ascent` and L2 Dusa queries
;;; directly join `verified_tier`, `verified_resolution`, and `verified_evidence`!
(def (materialize-evidence-chain-edb store base-edb)
  (let* ((reconciled (reconcile-search-evidence-chain store base-edb))
         (reusable (.ref reconciled 'reusable-claims))
         (conflict-keys (map car (.ref reconciled 'conflicts)))
         (claims (.ref store 'claims))
         (evidence-attrs
           (filter-map
             (lambda (cid)
               (let ((p (assoc cid claims)))
                 (and p
                      (let* ((cl (cdr p))
                             (tkey (.ref cl 'target-key))
                             (val (.ref cl 'value))
                             (ckind (.ref cl 'claim-kind)))
                        (and (not (member tkey conflict-keys))
                             (let ((akey (case ckind
                                           ((dusa-tier) "verified_tier")
                                           ((dusa-resolution) "verified_resolution")
                                           (else "verified_evidence"))))
                               (list tkey akey val)))))))
             reusable))
         (base-attrs (edb-rows base-edb 'attr))
         (merged-attrs (append base-attrs evidence-attrs)))
    (cons (cons 'attr merged-attrs)
          (filter (lambda (p) (not (eq? (car p) 'attr))) base-edb))))

;;; Evaluate any L1 `:gerbil-ascent` or L2 Dusa query over `base-edb` amplified
;;; with the store's live, truth-maintained M1 evidence chain!
(def (evaluate-search-dsl-with-evidence store query base-edb
                                        (registry SearchLanguageRegistryValue))
  (evaluate-search-dsl-over-edb
    query
    (materialize-evidence-chain-edb store base-edb)
    registry))

;;; Detect anti-LWW epistemic conflicts directly over surviving M1 claims (O(|claims|)),
;;; comparing distinct values within the SAME `(claim-kind, target-key)` domain.
(def (detect-store-m1-conflicts store)
  (let ((vals-by-key (make-hash-table))
        (conflicts '()))
    (for-each
      (lambda (pair)
        (let ((cl (cdr pair)))
          (when (eq? (.ref cl 'tier) 'M1-verified)
            (let* ((k (string-append (symbol->string (.ref cl 'claim-kind)) "::" (.ref cl 'target-key)))
                   (v (.ref cl 'value))
                   (prev (or (hash-get vals-by-key k) '())))
              (unless (member v prev)
                (hash-put! vals-by-key k (cons v prev)))))))
      (.ref store 'claims))
    (for-each
      (lambda (k)
        (let ((vs (sort-list (hash-get vals-by-key k) string<?)))
          (when (> (length vs) 1)
            (set! conflicts (cons (list k (car vs) (cadr vs)) conflicts)))))
      (sort-list (hash-keys vals-by-key) string<?))
    (reverse conflicts)))

;;; Fast O(|claims|) materialization when generation digest is unchanged (no files edited).
(def (materialize-live-store-attrs store base-edb conflicts)
  (let* ((conflict-keys (map car conflicts))
         (evidence-attrs
           (filter-map
             (lambda (pair)
               (let ((cl (cdr pair)))
                 (and (eq? (.ref cl 'tier) 'M1-verified)
                      (let ((tkey (.ref cl 'target-key))
                            (val (.ref cl 'value))
                            (ckind (.ref cl 'claim-kind)))
                        (and (not (member (string-append (symbol->string ckind) "::" tkey) conflict-keys))
                             (let ((akey (case ckind
                                           ((dusa-tier) "verified_tier")
                                           ((dusa-resolution) "verified_resolution")
                                           (else "verified_evidence"))))
                               (list tkey akey val)))))))
             (.ref store 'claims))))
    (if (null? evidence-attrs)
      base-edb
      (cons (cons 'attr (append (edb-rows base-edb 'attr) evidence-attrs))
            (filter (lambda (p) (not (eq? (car p) 'attr))) base-edb)))))

;;; Summarize verified layers and open cross-layer frontiers from accumulated M1 claims.
(def (compute-evidence-layers-and-frontiers store edb)
  (let* ((claim-paths
           (delete-duplicates/hash
             (append-map (lambda (pair) (.ref (cdr pair) 'file-deps))
                         (.ref store 'claims))))
         (covered-layers
           (sort-list
             (delete-duplicates/hash
               (filter-map extract-evidence-path-layer claim-paths))
             string<?))
         (open-frontiers '())
         (has-layer? (lambda (l) (and (member l covered-layers) #t)))
         (mentions-cabi?
           (ormap (lambda (pair)
                    (let* ((cl (cdr pair))
                           (tk (.ref cl 'target-key))
                           (deps (.ref cl 'file-deps)))
                      (or (string-contains-sub? tk "SearchDsl")
                          (string-contains-sub? tk "search_dsl")
                          (string-contains-sub? tk "search-dsl")
                          (string-contains-sub? tk "LinkedExports")
                          (ormap (lambda (d) (string-contains-sub? d "edb_bridge")) deps))))
                  (.ref store 'claims)))
         (edb-has-scheme-cabi?
           (ormap (lambda (f)
                    (and (>= (length f) 3)
                         (string-prefix-of? "scheme/c-abi/" (list-ref f 2))))
                  (edb-rows edb 'file))))
    (when (and mentions-cabi? (not (has-layer? "scheme/c-abi")))
      (set! open-frontiers
            (cons (if edb-has-scheme-cabi? "scheme/c-abi/search-dsl.ss" "scheme/c-abi")
                  open-frontiers)))
    (values covered-layers (reverse open-frontiers))))

(def (append-evidence-receipt-to-result res receipt-vec)
  (if (not (and (vector? res) (>= (vector-length res) 5) (eq? (vector-ref res 0) 'search-result)))
    res
    (let ((mode (vector-ref res 1))
          (len (vector-length res)))
      (cond
        ((eq? mode 'choice)
         (vector 'search-result 'choice
                 (vector-ref res 2)
                 (vector-ref res 3)
                 (vector-ref res 4)
                 (if (> len 5) (vector-ref res 5) #f)
                 receipt-vec))
        ((eq? mode 'ascent)
         (vector 'search-result 'ascent
                 (vector-ref res 2)
                 (vector-ref res 3)
                 (vector-ref res 4)
                 (if (> len 5) (vector-ref res 5) (length (vector-ref res 3)))
                 (if (> len 6) (vector-ref res 6) #f)
                 (if (> len 7) (vector-ref res 7) #f)
                 receipt-vec))
        (else res)))))

;;; Pure functional session-scoped Search DSL evaluation step wired to
;;; `evidence-chain.ss`:
;;;   1. Initializes or reconciles `store` against `generation` (O(1) fast path
;;;      when unchanged; full dual-fingerprint + ATMS + DE01 reconcile on drift).
;;;   2. Materializes live, non-conflicted M1 claims (`verified_tier`,
;;;      `verified_resolution`, `verified_evidence`) into Open EDB `attr`.
;;;   3. Executes `query` (via optional compiled `eval-fn` or `evaluate-search-dsl-over-edb`).
;;;   4. Harvests new oracle-backed M1 claims from L1 Ascent or L2 Dusa results
;;;      without collapsing multi-world `[branch]` ambiguity.
;;;   5. Projects a rooted `#(evidence-receipt ...)` vector attached to `search-result`.
(def (evaluate-session-search-dsl-step store query base-edb generation
                                       registry: (registry SearchLanguageRegistryValue)
                                       eval-fn: (eval-fn #f))
  (let* ((same-gen? (and store
                         (string? generation)
                         (> (string-length generation) 0)
                         (equal? (.ref store 'generation) generation)))
         (reconciled-store+conflicts
           (cond
             ((not store)
              (cons (make-search-evidence-store base-edb generation: generation) '()))
             (same-gen?
              (cons store (detect-store-m1-conflicts store)))
             (else
              (let ((rec (reconcile-search-evidence-chain store base-edb)))
                (cons (.cc (.ref rec 'next-store) 'generation generation)
                      (.ref rec 'conflicts))))))
         (active-store (car reconciled-store+conflicts))
         (pre-conflicts (cdr reconciled-store+conflicts))
         (amplified-edb (materialize-live-store-attrs active-store base-edb pre-conflicts))
         (raw-res (if eval-fn
                    (eval-fn amplified-edb)
                    (evaluate-search-dsl-over-edb query amplified-edb registry)))
         (mode (and (vector? raw-res) (>= (vector-length raw-res) 2) (vector-ref raw-res 1)))
         (harvested-info
           (cond
             ((eq? mode 'choice)
              (let ((h (harvest-choice-result-evidence active-store raw-res amplified-edb)))
                (vector (.ref h 'store)
                        (length (.ref h 'harvested))
                        (map car (.ref h 'ambiguous))
                        (.ref h 'world-count))))
             ((eq? mode 'ascent)
              (let ((h (harvest-ascent-result-evidence active-store raw-res amplified-edb)))
                (vector (.ref h 'store)
                        (length (.ref h 'harvested))
                        '()
                        0)))
             (else
              (vector active-store 0 '() 0))))
         (next-store (vector-ref harvested-info 0))
         (new-m1 (vector-ref harvested-info 1))
         (ambig-keys (vector-ref harvested-info 2))
         (world-count (vector-ref harvested-info 3))
         (post-conflicts (detect-store-m1-conflicts next-store))
         (total-m1 (length (filter (lambda (p) (eq? (.ref (cdr p) 'tier) 'M1-verified))
                                   (.ref next-store 'claims)))))
    (let-values (((covered-layers open-frontiers)
                  (compute-evidence-layers-and-frontiers next-store amplified-edb)))
      (let* ((status
               (cond
                 ((pair? post-conflicts) 'epistemic-conflict)
                 ((and (eq? mode 'choice) (= world-count 1)) 'unique-deduction)
                 ((or (pair? ambig-keys) (and (eq? mode 'choice) (> world-count 1))) 'competing-worlds)
                 ((and (null? open-frontiers) (> total-m1 0)) 'converged-evidence)
                 (else 'exploring)))
             (receipt-vec
               (vector 'evidence-receipt
                       (.ref next-store 'epoch)
                       total-m1
                       new-m1
                       (list->vector ambig-keys)
                       (length post-conflicts)
                       (list->vector covered-layers)
                       (list->vector open-frontiers)
                       status))
             (enriched-res (append-evidence-receipt-to-result raw-res receipt-vec)))
        (values enriched-res next-store)))))

;;; Verify a search evidence-chain causal trajectory through Marlin's Bi-Temporal
;;; Causality & ASCENT Temporal Lens (returning tamper-evident receipt and replay
;;; verification against candidate tampered histories).
(def (verify-evidence-causal-trajectory event-tuples
                                        forbidden-event-syms: (forbidden-event-syms '(poison_injected))
                                        tampered-event-tuples: (tampered-event-tuples #f))
  (let* ((build-specs
           (lambda (tuples)
             (map (lambda (t)
                    (make-temporal-governance-event
                      (list-ref t 0)
                      (list-ref t 1)
                      (list-ref t 2)
                      (list-ref t 3)
                      (list-ref t 4)
                      (list-ref t 5)
                      subject: "marlin-search-evidence-chain"))
                  tuples)))
         (specs (build-specs event-tuples))
         (root-id (.ref (car specs) 'event-id))
         (max-pos (apply max (map (lambda (s) (.ref s 'logical-pos)) specs)))
         (min-val (apply min (map (lambda (s) (.ref s 'valid-coord)) specs)))
         (max-val (apply max (map (lambda (s) (.ref s 'valid-coord)) specs)))
         (max-know (apply max (map (lambda (s) (.ref s 'knowledge-coord)) specs)))
         (horizon (min 128 (max 1 (length specs))))
         (receipt (evaluate-temporal-causal-governance
                    specs root-id max-pos min-val (+ max-val 1) max-know horizon
                    forbidden-event-syms: forbidden-event-syms))
         (tamper-replay
           (and tampered-event-tuples
                (verify-temporal-causal-governance
                  receipt (build-specs tampered-event-tuples)
                  max-pos min-val (+ max-val 1) max-know horizon))))
    (.o bind-these: #f
        receipt: receipt
        admitted?: (.ref receipt 'admitted?)
        forbidden-hits: (.ref receipt 'forbidden-hits)
        proof-digest: (.ref receipt 'proof-digest)
        tamper-replay-status: tamper-replay)))
