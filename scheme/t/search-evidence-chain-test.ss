;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; End-to-End Scenario B (Dusa + ASCENT Unified Search/Disambiguation/Solving)
;;; Compound Evidence-Chain ("越搜越强") & Adversarial Defense Test Suite.
;;;
;;; Verifies Marlin's downstream implementation of upstream QNT models:
;;;   1. Scenario B1: Same-name callsite disambiguation (`resolve`: `closed`
;;;      same-module deduction, `open` cross-module branching, `closed ()` forbid
;;;      `in-test-file`) -> oracle-backed M1 evidence harvesting.
;;;   2. Scenario B2: Architecture placement constraint solving (`tier`: `resident`
;;;      vs `provider`, `hot-path-module` forbid `provider`) -> compound search
;;;      amplification ("越搜越强") where Turn 2 L1/L2 queries directly consume
;;;      accumulated `verified_tier` and `verified_resolution` M1 facts.
;;;   3. Adversarial 1 — Comment-Only Edit vs Structural AST Drift:
;;;      comment-only edit invalidates ONLY L0 content-sensitive cache while
;;;      preserving 100% of M1 structural & Dusa evidence (epoch unchanged);
;;;      AST structural edit selectively invalidates affected claims + transitive
;;;      dependents while preserving unaffected files!
;;;   4. Adversarial 2 — DE01 Negative-Domain Expansion:
;;;      new file/symbol entering a watched scope immediately invalidates
;;;      universal/negative claims over that scope while keeping frozen historical
;;;      cuts and unrelated positive M1 proofs intact!
;;;   5. Adversarial 3 — KV-Cache Epoch Fencing, Pinned-Constraint Amnesia &
;;;      Untrusted Integrity Laundering (`SearchEvidenceChainIntegrity.qnt`).
;;;   6. Adversarial 4 — Memory/Evidence-Chain Poisoning, M2 Quarantine, ATMS
;;;      Multi-Justification Survival vs Zombie Retraction, & Forensic Poison
;;;      Immunization Tombstones (`DurableMemoryTms.qnt` / B8).
;;;   7. Adversarial 5 — Compromised Principal Cascade + Bi-Temporal Causality
;;;      Tamper Replay Rejection (`scheme/policy/governance.ss`).
;;;   8. Adversarial 6 — Epistemic Conflict Preservation (Anti-LWW).
(import :std/test
        (only-in :clan/poo/object .ref)
        :marlin-agent-core/search-dsl/objects
        :marlin-agent-core/search-dsl/funcs
        :marlin-agent-core/search-dsl/evidence-chain)
(export search-evidence-chain-test)

(def +base-scenario-b-edb+
  '((file . ((1 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs")
             (2 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
             (3 "rust" "crates/marlin-agent-tooling/src/search_tool.rs")
             (4 "rust" "crates/marlin-agent-tooling/tests/mock_loader.rs")))
    (entity . (("e-runner-exec"   1 "rust" "function" "fn_item"     "execute_dsl"  10)
               ("e-runner-load"   1 "rust" "function" "fn_item"     "load_index"   25)
               ("e-codec-decode"  2 "rust" "function" "fn_item"     "decode_rows"  12)
               ("e-tool-dispatch" 3 "rust" "function" "fn_item"     "dispatch_req" 18)
               ("e-tool-load"     3 "rust" "function" "fn_item"     "load_index"   40)
               ("e-test-load"     4 "rust" "function" "fn_item"     "load_index"   15)))
    (edge . (("e-runner-exec"   "load_index" "calls" "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs" 14)
             ("e-codec-decode"  "load_index" "calls" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs"  20)))
    (attr . ())))

(def search-evidence-chain-test
  (test-suite "Scenario B Unified Evidence Chain, Compound Search Amplification & Adversarial Defense"

    (test-case "Scenario B1 + B2 + Compound Amplification: Dusa disambiguation & placement harvest M1 facts consumed directly by Turn-2 queries"
      (let* ((store0 (make-search-evidence-store
                       +base-scenario-b-edb+
                       watched-scopes: '("scope:crates/marlin-agent-search-engine/src/edb_bridge/")
                       frozen-cuts: '(("cut-v1" "sha256:frozen-history-root-001"))))
             ;; Scenario B1: Callsite disambiguation (`resolve`):
             ;;   - `e-runner-exec@L14:load_index` has same-module def `e-runner-load` -> CLOSED deduction!
             ;;   - `e-codec-decode@L20:load_index` has cross-module candidates `e-runner-load`, `e-tool-load`,
             ;;     and `e-test-load` (in-test-file FORBIDDEN!) -> 2 clean non-test worlds (branch ambiguity preserved!).
             (disambig-query
               "(search-choice-program
                  (use rust)
                  (choice resolve (callsite target))
                  (rule (closed (resolve ?call) (?def))
                        (ref-at ?call ?name)
                        (same-module-def ?call ?name ?def))
                  (rule (open (resolve ?call) ?def)
                        (ref-at ?call ?name)
                        (workspace-def ?name ?def))
                  (rule (closed (resolve ?call) ())
                        (resolve ?call ?def)
                        (in-test-file ?def))
                  (solve resolve state-budget: 64 provenance?: #t))")
             (h1 (harvest-dusa-scenario-b-evidence store0 disambig-query +base-scenario-b-edb+
                                                   claim-prefix: "b1"))
             (store1 (.ref h1 'store))
             ;; Scenario B2: Architecture placement (`tier`) on `edb_bridge/` hot-path modules ->
             ;;   all 2 hot-path modules deduced as `resident` in 1 SAT world!
             (tier-query
               "(search-choice-program
                  (use rust)
                  (scope path-prefix: \"crates/marlin-agent-search-engine/src/edb_bridge/\" limit: 6)
                  (choice tier (mod level))
                  (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m))
                  (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider))
                  (solve tier state-budget: 64 provenance?: #t))")
             (h2 (harvest-dusa-scenario-b-evidence
                   store1 tier-query +base-scenario-b-edb+
                   claim-prefix: "b2"
                   neg-domains: '("scope:crates/marlin-agent-search-engine/src/edb_bridge/")))
             (store2 (.ref h2 'store))
             ;; Turn 2 Compound Search Amplification ("越搜越强"):
             ;;   An L1 Ascent query directly joins `verified_tier` + `verified_resolution`
             ;;   accumulated from Turn 1 WITHOUT re-running Dusa!
             (turn2-ascent
               (evaluate-search-dsl-with-evidence
                 store2
                 "(ascent-fragment
                    ((answer mod lvl call target)
                     <-- (verified_tier mod lvl)
                         (verified_resolution call target)))"
                 +base-scenario-b-edb+
                 SearchLanguageRegistryValue))
             (turn2-rows (vector-ref turn2-ascent 3)))
        ;; B1 disambiguation: 2 SAT worlds (test mock excluded!), same-module callsite harvested as M1,
        ;; cross-module ambiguous callsite preserved in `ambiguous` (never LWW-collapsed!).
        (check (.ref h1 'world-count) => 2)
        (check (length (.ref h1 'harvested)) => 1)
        (check (length (.ref h1 'ambiguous)) => 1)
        (check (not (member "e-test-load" (cadr (car (.ref h1 'ambiguous))))) => #t)
        ;; B2 placement: 1 SAT world, both hot-path files deduced as M1 `resident`.
        (check (.ref h2 'world-count) => 1)
        (check (length (.ref h2 'harvested)) => 2)
        ;; Turn 2 directly builds on Turn 1's M1 evidence chain (2 verified_tier * 1 verified_resolution = 2 rows!).
        (check (length turn2-rows) => 2)
        (check (and (member '("crates/marlin-agent-search-engine/src/edb_bridge/runner.rs"
                              "resident"
                              "e-runner-exec@L14:load_index"
                              "e-runner-load")
                            turn2-rows)
                    #t)
               => #t)))

    (test-case "Adversarial 1 + 2: Comment-only edit preserves M1 structural reuse; structural AST drift & DE01 negative-domain expansion selectively invalidate while preserving frozen cuts"
      (let* ((store0 (make-search-evidence-store
                       +base-scenario-b-edb+
                       watched-scopes: '("scope:crates/marlin-agent-search-engine/src/edb_bridge/")
                       frozen-cuts: '(("cut-hist-1" "sha256:immutable-cut-001"))))
             ;; Admit 3 claims:
             ;;   - c-l0-runner: content-sensitive L0 text cache on runner.rs
             ;;   - c-struct-runner: structural M1 claim on runner.rs
             ;;   - c-struct-codec: structural M1 claim on codec.rs (unaffected by runner.rs drift!)
             ;;   - c-neg-scope: universal/negative claim over edb_bridge/ scope (DE01)
             (c-l0 (make-search-evidence-claim
                     "c-l0-runner" 'ascent-fact "l0:runner" "line10"
                     file-deps: '("crates/marlin-agent-search-engine/src/edb_bridge/runner.rs")
                     content-sensitive?: #t
                     oracle-receipt: "sha256:oracle-l0"))
             (c-sr (make-search-evidence-claim
                     "c-struct-runner" 'dusa-tier
                     "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs" "resident"
                     file-deps: '("crates/marlin-agent-search-engine/src/edb_bridge/runner.rs")
                     content-sensitive?: #f
                     oracle-receipt: "sha256:oracle-sr"))
             (c-sc (make-search-evidence-claim
                     "c-struct-codec" 'dusa-tier
                     "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs" "resident"
                     file-deps: '("crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
                     content-sensitive?: #f
                     oracle-receipt: "sha256:oracle-sc"))
             (c-neg (make-search-evidence-claim
                      "c-neg-scope" 'negative-invariant
                      "no-external-provider-in-edb-bridge" "true"
                      neg-domains: '("scope:crates/marlin-agent-search-engine/src/edb_bridge/")
                      oracle-receipt: "sha256:oracle-neg"))
             (s1 (.ref (admit-search-evidence-claim store0 c-l0 edb: +base-scenario-b-edb+) 'store))
             (s2 (.ref (admit-search-evidence-claim s1 c-sr edb: +base-scenario-b-edb+) 'store))
             (s3 (.ref (admit-search-evidence-claim s2 c-sc edb: +base-scenario-b-edb+) 'store))
             (s4 (.ref (admit-search-evidence-claim s3 c-neg edb: +base-scenario-b-edb+) 'store))

             ;; Phase A: COMMENT-ONLY edit on runner.rs (line numbers shift + content hash changes, AST identical!):
             (comment-edited-edb
               (cons (cons 'entity
                           (map (lambda (ent)
                                  (if (= (list-ref ent 1) 1)
                                    (list (list-ref ent 0) (list-ref ent 1) (list-ref ent 2)
                                          (list-ref ent 3) (list-ref ent 4) (list-ref ent 5)
                                          (+ (list-ref ent 6) 5))
                                    ent))
                                (cdr (assq 'entity +base-scenario-b-edb+))))
                     (filter (lambda (p) (not (eq? (car p) 'entity))) +base-scenario-b-edb+)))
             (rec-comment
               (reconcile-search-evidence-chain
                 s4 comment-edited-edb
                 content-overrides: '(("crates/marlin-agent-search-engine/src/edb_bridge/runner.rs" . "sha256:comment-edit-v2"))))

             ;; Phase B: STRUCTURAL AST edit on runner.rs (new function `bypass_hot_path` added!)
             ;;          -> BOTH runner.rs structural drift AND DE01 negative-domain expansion!
             (struct-edited-edb
               (cons (cons 'entity
                           (cons '("e-runner-new" 1 "rust" "function" "fn_item" "bypass_hot_path" 99)
                                 (cdr (assq 'entity +base-scenario-b-edb+))))
                     (filter (lambda (p) (not (eq? (car p) 'entity))) +base-scenario-b-edb+)))
             (rec-struct
               (reconcile-search-evidence-chain
                 (.ref rec-comment 'next-store)
                 struct-edited-edb
                 kv-entries: '(("kv-stale-runner" "c-struct-runner" 1 #t #f)
                               ("kv-stale-codec"  "c-struct-codec"  1 #t #f)
                               ("kv-fresh-codec"  "c-struct-codec"  2 #t #f)
                               ("kv-amnesia"      "c-struct-codec"  2 #f #f)
                               ("kv-laundered"    "c-struct-codec"  2 #t #t)))))
        ;; Phase A (comment-only): epoch stays 1; L0 cache invalidated; ALL 3 structural/negative claims REUSED!
        (check (.ref rec-comment 'active-epoch) => 1)
        (check (null? (.ref rec-comment 'structural-drifted)) => #t)
        (check (and (member '("c-l0-runner" "content-drift-l0") (.ref rec-comment 'invalidated-claims)) #t) => #t)
        (check (and (member "c-struct-runner" (.ref rec-comment 'reusable-claims))
                    (member "c-struct-codec" (.ref rec-comment 'reusable-claims))
                    (member "c-neg-scope" (.ref rec-comment 'reusable-claims))
                    #t)
               => #t)
        ;; Phase B (structural + DE01): epoch advances to 2; runner & neg-scope invalidated;
        ;; codec.rs 100% REUSED; frozen history cut 100% PRESERVED; bi-temporal digest emitted!
        (check (.ref rec-struct 'active-epoch) => 2)
        (check (and (member '("c-struct-runner" "structural-drift") (.ref rec-struct 'invalidated-claims)) #t) => #t)
        (check (and (member '("c-neg-scope" "neg-domain-expanded") (.ref rec-struct 'invalidated-claims)) #t) => #t)
        (check (.ref rec-struct 'reusable-claims) => '("c-struct-codec"))
        (check (.ref rec-struct 'frozen-cuts-preserved) => '(("cut-hist-1" "sha256:immutable-cut-001")))
        (check (and (.ref rec-struct 'bitemporal-receipt)
                    (string? (.ref (.ref rec-struct 'bitemporal-receipt) 'invalidation-digest)))
               => #t)
        ;; Adversarial 3 (KV-Cache Fencing + Distillation Amnesia + Integrity Laundering):
        (check (.ref rec-struct 'kv-admitted) => '("kv-fresh-codec"))
        (check (and (member '("kv-stale-runner" "kv-stale-epoch") (.ref rec-struct 'kv-rejected))
                    (member '("kv-stale-runner" "kv-zombie-claim") (.ref rec-struct 'kv-rejected))
                    (member '("kv-stale-codec" "kv-stale-epoch") (.ref rec-struct 'kv-rejected))
                    (member '("kv-amnesia" "distillation-pinned-amnesia") (.ref rec-struct 'kv-rejected))
                    (member '("kv-laundered" "integrity-laundering") (.ref rec-struct 'kv-rejected))
                    #t)
               => #t)))

    (test-case "Adversarial 4 + 5 + 6: M2 quarantine, ATMS multi-justification survival, poison tombstone anti-repoisoning, principal cascade, anti-LWW conflict & bi-temporal tamper replay"
      (let* ((store0 (make-search-evidence-store +base-scenario-b-edb+))
             ;; 1. Adversarial M2 untrusted prompt-injection claim -> QUARANTINED (never enters M1!)
             (poison-untrusted
               (make-search-evidence-claim
                 "c-prompt-inj" 'ascent-fact "policy:bypass" "allow-bash"
                 proposer: "web-snippet"
                 source-integrity: 'untrusted-external))
             (adm-untrusted (admit-search-evidence-claim store0 poison-untrusted))
             ;; 2. Adversarial unverified single-agent hallucination (no oracle, quorum < 2) -> QUARANTINED!
             (halluc-m2
               (make-search-evidence-claim
                 "c-halluc" 'ascent-fact "call:fake" "ghost_fn"
                 proposer: "agent-a"
                 attesters: '("agent-a"))) ;; self-attestation MUST NOT count!
             (adm-halluc (admit-search-evidence-claim (.ref adm-untrusted 'store) halluc-m2))
             ;; 3. Compromised root (initially quorum-promoted by rogue agent-x + 2 attesters, no oracle)
             ;;    + Clean independent oracle root
             ;;    + Derived Zombie (sole support: compromised root)
             ;;    + Derived ATMS Survivor (disjunctive support: [compromised] OR [clean-oracle]!)
             (c-poison-root
               (make-search-evidence-claim
                 "c-poison-root" 'ascent-fact "arch:edge" "sus-link"
                 proposer: "agent-compromised"
                 attesters: '("w1" "w2")))
             (c-clean-root
               (make-search-evidence-claim
                 "c-clean-root" 'ascent-fact "arch:edge-clean" "verified-link"
                 proposer: "dusa-oracle"
                 oracle-receipt: "sha256:oracle-clean-root"
                 source-integrity: 'trusted-oracle))
             (c-zombie
               (make-search-evidence-claim
                 "c-derived-zombie" 'derived-chain "plan:zombie" "use-sus"
                 justifications: '(("c-poison-root"))
                 oracle-receipt: "sha256:deriv-1"))
             (c-survivor
               (make-search-evidence-claim
                 "c-derived-survivor" 'derived-chain "plan:survivor" "safe-route"
                 justifications: '(("c-poison-root") ("c-clean-root"))
                 oracle-receipt: "sha256:deriv-2"))
             ;; 4. Anti-LWW Epistemic Conflict: two independent oracle claims with incompatible values on same target!
             (c-conf-a
               (make-search-evidence-claim
                 "c-conf-a" 'dusa-tier "mod:contested.rs" "provider"
                 oracle-receipt: "sha256:conf-a" source-integrity: 'trusted-oracle))
             (c-conf-b
               (make-search-evidence-claim
                 "c-conf-b" 'dusa-tier "mod:contested.rs" "resident"
                 oracle-receipt: "sha256:conf-b" source-integrity: 'trusted-oracle))
             (s1 (.ref (admit-search-evidence-claim (.ref adm-halluc 'store) c-poison-root) 'store))
             (s2 (.ref (admit-search-evidence-claim s1 c-clean-root) 'store))
             (s3 (.ref (admit-search-evidence-claim s2 c-zombie) 'store))
             (s4 (.ref (admit-search-evidence-claim s3 c-survivor) 'store))
             (s5 (.ref (admit-search-evidence-claim s4 c-conf-a) 'store))
             (s6 (.ref (admit-search-evidence-claim s5 c-conf-b) 'store))
             ;; Retract c-poison-root + revoke agent-compromised!
             (rec (reconcile-search-evidence-chain
                    s6 +base-scenario-b-edb+
                    retract-poison-ids: '("c-poison-root")
                    revoke-principals: '("agent-compromised")))
             (s-after (.ref rec 'next-store))
             ;; Attempt cross-session RE-POISONING with exact same claim content digest!
             (repoison-claim
               (make-search-evidence-claim
                 "c-poison-reborn" 'ascent-fact "arch:edge" "sus-link"
                 proposer: "clean-agent"
                 oracle-receipt: "sha256:forged-reentry"))
             (adm-repoison (admit-search-evidence-claim s-after repoison-claim))
             ;; Materialized EDB MUST exclude contested conflict key `mod:contested.rs` (anti-LWW)!
             (mat-edb (materialize-evidence-chain-edb s-after +base-scenario-b-edb+))
             (mat-attrs (cdr (assq 'attr mat-edb)))
             ;; Bi-Temporal Causality Trajectory Proof + Tamper Replay Negative Control:
             (clean-traj
               '(("ev-1" search_init   1 ()       1 1)
                 ("ev-2" dusa_solved   2 ("ev-1") 2 2)
                 ("ev-3" evidence_m1   3 ("ev-2") 3 3)))
             (tampered-traj
               '(("ev-1" search_init   1 ()       1 1)
                 ("ev-2" dusa_solved   2 ("ev-1") 99 2)
                 ("ev-3" evidence_m1   3 ("ev-2") 3 3)))
             (poisoned-traj
               '(("ev-1" search_init     1 ()       1 1)
                 ("ev-2" poison_injected 2 ("ev-1") 2 2)
                 ("ev-3" evidence_m1     3 ("ev-2") 3 3)))
             (causal-ok (verify-evidence-causal-trajectory
                          clean-traj tampered-event-tuples: tampered-traj))
             (causal-poison (verify-evidence-causal-trajectory poisoned-traj)))
        ;; M2 quarantine checks:
        (check (.ref adm-untrusted 'admitted?) => #f)
        (check (.ref adm-untrusted 'reason) => 'untrusted-external-source)
        (check (.ref adm-halluc 'admitted?) => #f)
        (check (.ref adm-halluc 'reason) => 'missing-oracle-or-quorum)
        ;; ATMS retraction: zombie retracted, dual-support survivor preserved!
        (check (and (member '("c-derived-zombie" "atms-all-justifications-lost")
                            (.ref rec 'invalidated-claims))
                    #t)
               => #t)
        (check (and (member "c-derived-survivor" (.ref rec 'reusable-claims)) #t) => #t)
        (check (.ref rec 'atms-preserved-claims) => '("c-derived-survivor"))
        ;; Poison tombstone permanently blocks cross-session re-ingestion!
        (check (.ref adm-repoison 'admitted?) => #f)
        (check (.ref adm-repoison 'reason) => 'poison-tombstone-blocked)
        ;; Anti-LWW Epistemic Conflict preserved and NEVER silently materialized into EDB!
        (check (.ref rec 'conflicts) => '(("mod:contested.rs" "provider" "resident")))
        (check (assoc "mod:contested.rs" mat-attrs) => #f)
        ;; Bi-Temporal Causality admits clean history, rejects tampered replay, rejects poison ancestor!
        (check (.ref causal-ok 'admitted?) => #t)
        (check (.ref causal-ok 'tamper-replay-status) => 'invalid)
        (check (.ref causal-poison 'admitted?) => #f)
        (check (.ref causal-poison 'forbidden-hits) => '(poison_injected))))

    (test-case "Live Session Step: Turn-1 Dusa harvests M1 receipt & Turn-2 Ascent consumes verified_tier with open-frontier detection"
      (let* ((tier-query
               "(search-choice-program
                  (use rust)
                  (scope path-prefix: \"crates/marlin-agent-search-engine/src/edb_bridge/\" limit: 6)
                  (choice tier (mod level))
                  (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m))
                  (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider))
                  (solve tier state-budget: 64 provenance?: #t))")
             (ascent-query
               "(ascent-fragment
                  ((answer mod lvl)
                   <-- (verified_tier mod lvl)))"))
        (let-values (((res1 store1)
                      (evaluate-session-search-dsl-step
                        #f tier-query +base-scenario-b-edb+ "gen-session-1")))
          (let* ((receipt1 (vector-ref res1 6)))
            (check (vector-ref receipt1 0) => 'evidence-receipt)
            (check (vector-ref receipt1 1) => 1)
            (check (vector-ref receipt1 2) => 2)
            (check (vector-ref receipt1 3) => 2)
            (check (vector-ref receipt1 8) => 'unique-deduction)
            ;; Turn 2 reuses same generation (O(1) fast path) and directly joins Turn 1 M1 verified_tier!
            (let-values (((res2 _store2)
                          (evaluate-session-search-dsl-step
                            store1 ascent-query +base-scenario-b-edb+ "gen-session-1")))
              (let ((rows2 (vector-ref res2 3))
                    (receipt2 (vector-ref res2 8)))
                (check (length rows2) => 2)
                (check (vector-ref receipt2 0) => 'evidence-receipt)
                (check (>= (vector-ref receipt2 2) 2) => #t)
                ;; `edb_bridge` reference surfaces uncovered `scheme/c-abi` cross-layer frontier!
                (check (and (member "scheme/c-abi" (vector->list (vector-ref receipt2 7))) #t) => #t)))))))))
