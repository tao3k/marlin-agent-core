;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Qualification suite for Marlin's Higher-Order Governance Plane:
;;;   1. Atomic POO Governance Profile composition (`agentic-ai-lab/modules/governance`)
;;;   2. `gerbil-ascent` Higher-Order Typed Fixpoint (`relational-typed-fix`) transitive closure
;;;   3. `gerbil-ascent` Stratified Datalog CaMeL taint paths + human sanitizer cut
;;;   4. `gerbil-ascent` Stratified Multi-Hop OBO revocation + 5-scope exfiltration + breaker fencing
;;;   5. `agentic-ai-lab/modules/temporal-causality` Bi-Temporal (`valid-time` x `knowledge-time`)
;;;      causal cuts + tamper-evident ASCENT replay verification
;;;   6. `agentic-ai-lab/modules/governance/policy/strict-mediation` one-shot SHA-256 token chain
;;;   7. `agentic-ai-lab/modules/authorization/providers/cedar` dynamic Cedar synthesis &
;;;      proof-bound `CedarAuthoritySnapshot` / `CedarAuthorizationRequest` runtime projections.

(import :std/test
        (only-in :clan/poo/object .get .o .ref)
        (only-in :tao3k/agentic-ai-lab/modules/governance/interface
                 agentic-ai-lab-governance-source
                 agentic-ai-lab-governance-precondition
                 agentic-ai-lab-governance-threat)
        (only-in :tao3k/agentic-ai-lab/modules/governance/policy/strict-mediation
                 agentic-ai-lab-strict-mediation-state)
        "../policy/higher-order-governance")

(export marlin-higher-order-governance-test)

(def (mk-threat id sev phase mitigations: (mits '()))
  (agentic-ai-lab-governance-threat
   id sev phase
   (list (agentic-ai-lab-governance-precondition (string-append id ".pre") #t "observed-in-model"))
   mitigations: mits))

(def marlin-higher-order-governance-test
  (test-suite "Marlin Higher-Order Governance (Atomic POO x ASCENT x Temporal Causality x Cedar)"

    ;; -----------------------------------------------------------------------
    ;; 1. Atomic POO Governance Profile Composition & Monotonic Threat Blocking
    ;; -----------------------------------------------------------------------
    (test-case "atomic POO composition preserves unresolved critical threats monotonically"
      (let* ((src-breaker (agentic-ai-lab-governance-source
                           "marlin.breaker.spec"
                           "dev/quint/threat/MarlinBackendExecutionBreakerOverlay.qnt"
                           'quint))
             (src-ctx (agentic-ai-lab-governance-source
                       "marlin.context.spec"
                       "dev/quint/threat/MarlinBackendSharedContextOverlay.qnt"
                       'quint))
             (p-breaker-mitigated
              (make-marlin-governance-profile
               "marlin.profile.execution-breaker" "1.0.0" "tao3k-security"
               "tm.execution-breaker"
               (list (mk-threat "breaker.exec-while-tripped" 'critical 'mitigated
                                mitigations: '("rust-execution-breaker-latch")))
               source-list: (list src-breaker)))
             (p-ctx-exposed
              (make-marlin-governance-profile
               "marlin.profile.shared-context" "1.0.0" "tao3k-security"
               "tm.shared-context"
               (list (mk-threat "context.private-exfiltration" 'critical 'exposed))
               source-list: (list src-ctx)))
             (composed-blocked
              (compose-governance-profiles
               "marlin.profile.composed" "1.0.0" "tao3k-security"
               (list p-breaker-mitigated p-ctx-exposed)))
             (eval-blocked (evaluate-marlin-governance-profile composed-blocked (.o scene: 'tool-dispatch))))
        (check (.ref eval-blocked 'handoff-ready?) => #f)
        (check (.ref eval-blocked 'unresolved-threats) => '("context.private-exfiltration"))
        (check (.ref eval-blocked 'runtime-executed?) => #f)))

    ;; -----------------------------------------------------------------------
    ;; 2. ASCENT Higher-Order Relational Fixpoint (`relational-typed-fix`)
    ;; -----------------------------------------------------------------------
    (test-case "gerbil-ascent higher-order relational-typed-fix computes exact multi-hop OBO chain"
      (let* ((domain '(human-alice agent-1 agent-2 tool-shell))
             (edges '((human-alice agent-1)
                      (agent-1 agent-2)
                      (agent-2 tool-shell)))
             (res (solve-higher-order-transitive-closure domain edges))
             (rows (.ref res 'closure-rows)))
        ;; Multi-hop transitive reachability derived purely by higher-order fixpoint
        (check (and (member '(human-alice agent-2) rows) #t) => #t)
        (check (and (member '(human-alice tool-shell) rows) #t) => #t)
        (check (and (member '(agent-1 tool-shell) rows) #t) => #t)
        (check (string-prefix? "sha256:" (.ref res 'proof-digest)) => #t)))

    ;; -----------------------------------------------------------------------
    ;; 3. ASCENT Stratified Datalog CaMeL Taint Paths & Human Sanitizer Cut
    ;; -----------------------------------------------------------------------
    (test-case "gerbil-ascent taint-paths detects multi-hop prompt hijack and clears after human sanitizer"
      (let* ((edges '((chatroom-ext shared-ctx)
                      (shared-ctx llm-prompt)
                      (llm-prompt human-gate)
                      (human-gate os-shell-tool)))
             (sources '((chatroom-ext)))
             (sinks '((os-shell-tool)))
             ;; Without human gate sanitizer -> multi-hop taint reaches OS shell sink!
             (unsanitized (solve-ascent-camel-taint-paths edges sources sinks '()))
             ;; With verified human gate sanitizer -> stratified negation cuts taint path!
             (sanitized (solve-ascent-camel-taint-paths edges sources sinks '((human-gate)))))
        (check (.ref unsanitized 'clean?) => #f)
        (check (.ref unsanitized 'taint-alerts) => '((chatroom-ext os-shell-tool)))
        (check (.ref sanitized 'clean?) => #t)
        (check (.ref sanitized 'taint-alerts) => '())))

    ;; -----------------------------------------------------------------------
    ;; 4. ASCENT Multi-Hop Revocation, 5-Scope Exfiltration & Breaker Fencing
    ;; -----------------------------------------------------------------------
    (test-case "gerbil-ascent fixpoint propagates transitive OBO revocation and catches private->channel leak"
      (let* ((res-leak
              (solve-ascent-delegation-and-scope-governance
               delegation-edges: '((alice agent-a) (agent-a agent-sub))
               revoked-actors: '((alice))
               context-entries: '((secret-note private) (public-doc workspace))
               context-flows: '((secret-note prompt-draft) (prompt-draft channel-prompt))
               target-scopes: '((channel-prompt channel))
               declassified-entries: '()
               tripped-breakers: '((estop-main))
               breaker-tools: '((estop-main shell-exec) (estop-main disk-write))))
             (res-declassified
              (solve-ascent-delegation-and-scope-governance
               delegation-edges: '((alice agent-a) (agent-a agent-sub))
               revoked-actors: '((alice))
               context-entries: '((secret-note private))
               context-flows: '((secret-note prompt-draft) (prompt-draft channel-prompt))
               target-scopes: '((channel-prompt channel))
               declassified-entries: '((secret-note))
               tripped-breakers: '()
               breaker-tools: '((estop-main shell-exec)))))
        ;; Revoking root human `alice` transitively fences both `agent-a` AND `agent-sub`!
        (check (and (member '(alice) (.ref res-leak 'fenced-actors)) #t) => #t)
        (check (and (member '(agent-a) (.ref res-leak 'fenced-actors)) #t) => #t)
        (check (and (member '(agent-sub) (.ref res-leak 'fenced-actors)) #t) => #t)
        ;; Multi-hop private -> prompt-draft -> channel-prompt detected fail-closed!
        (check (.ref res-leak 'scope-exfiltration-alerts) => '((secret-note channel-prompt)))
        ;; Tripped breaker fences both shell-exec and disk-write!
        (check (and (member '(shell-exec) (.ref res-leak 'breaker-blocked-tools)) #t) => #t)
        (check (and (member '(disk-write) (.ref res-leak 'breaker-blocked-tools)) #t) => #t)
        (check (.ref res-leak 'clean?) => #f)
        ;; Once explicitly declassified and breaker armed, exfiltration alert clears cleanly!
        (check (.ref res-declassified 'scope-exfiltration-alerts) => '())
        (check (.ref res-declassified 'clean?) => #t)))

    ;; -----------------------------------------------------------------------
    ;; 5. Bi-Temporal Causality (`valid-time` x `knowledge-time`) & Tamper Detection
    ;; -----------------------------------------------------------------------
    (test-case "bi-temporal causal cut verifies lineage and detects retroactive knowledge-time tampering"
      (let* ((events-ok
              (list (make-temporal-governance-event "E-AUTH" 'auth 0 '() 0 0)
                    (make-temporal-governance-event "E-GATE" 'gate 1 '("E-AUTH") 1 1)
                    (make-temporal-governance-event "E-EXEC" 'exec 2 '("E-GATE") 2 2)))
             (events-tampered
              (list (make-temporal-governance-event "E-AUTH" 'auth 0 '() 0 0)
                    ;; Adversary retroactively alters knowledge-time of human gate!
                    (make-temporal-governance-event "E-GATE" 'gate 1 '("E-AUTH") 1 3)
                    (make-temporal-governance-event "E-EXEC" 'exec 2 '("E-GATE") 2 2)))
             (events-with-revoke
              (list (make-temporal-governance-event "E-AUTH" 'auth 0 '() 0 0)
                    (make-temporal-governance-event "E-REVOKE" 'revoked 1 '("E-AUTH") 1 1)
                    (make-temporal-governance-event "E-EXEC" 'exec 2 '("E-REVOKE") 2 2)))
             (cut-ok (evaluate-temporal-causal-governance
                      events-ok "E-AUTH" 2 0 5 4 3
                      forbidden-event-syms: '(revoked)))
             (cut-revoked (evaluate-temporal-causal-governance
                           events-with-revoke "E-AUTH" 2 0 5 4 3
                           forbidden-event-syms: '(revoked))))
        (check (.ref cut-ok 'admitted?) => #t)
        (check (.ref cut-ok 'temporal-status) => 'complete)
        (check (.ref cut-ok 'replay-status) => 'valid)
        (check (verify-temporal-causal-governance cut-ok events-ok 2 0 5 4 3) => 'valid)
        ;; Tampering with historical knowledge-time invalidates the ASCENT causal receipt!
        (check (verify-temporal-causal-governance cut-ok events-tampered 2 0 5 4 3) => 'invalid)
        ;; Upstream revocation event inside the causal DAG blocks admission!
        (check (.ref cut-revoked 'admitted?) => #f)
        (check (.ref cut-revoked 'forbidden-hits) => '(revoked))))

    ;; -----------------------------------------------------------------------
    ;; 6. Strict One-Shot AuthorizedEffectToken Mediation & Cedar Proof-Bound Snapshot
    ;; -----------------------------------------------------------------------
    (test-case "full pipeline synthesizes proof-bound Cedar snapshot and enforces one-shot strict mediation"
      (let* ((p-safe
              (make-marlin-governance-profile
               "marlin.profile.backend-unified" "1.0.0" "tao3k-security"
               "tm.backend-unified"
               (list (mk-threat "breaker.exec-while-tripped" 'critical 'mitigated
                                mitigations: '("rust-execution-breaker"))
                     (mk-threat "context.private-leak" 'critical 'mitigated
                                mitigations: '("rust-5scope-ledger")))))
             (gov-ok (evaluate-marlin-governance-profile p-safe (.o scene: 'dispatch)))
             (taint-ok (solve-ascent-camel-taint-paths
                        '((chatroom-ext human-gate) (human-gate tool-exec))
                        '((chatroom-ext)) '((tool-exec)) '((human-gate))))
             (taint-bad (solve-ascent-camel-taint-paths
                         '((chatroom-ext tool-exec))
                         '((chatroom-ext)) '((tool-exec)) '()))
             (fix-ok (solve-ascent-delegation-and-scope-governance
                      delegation-edges: '((alice agent-revoked-child))
                      revoked-actors: '((alice))))
             (temporal-ok (evaluate-temporal-causal-governance
                           (list (make-temporal-governance-event "E-AUTH" 'auth 0 '() 0 0)
                                 (make-temporal-governance-event "E-EXEC" 'exec 1 '("E-AUTH") 1 1))
                           "E-AUTH" 1 0 5 4 3
                           forbidden-event-syms: '(revoked)))
             ;; Fail-closed check: ASCENT taint alert MUST block Cedar snapshot synthesis!
             (denied-snap (synthesize-marlin-cedar-authority-snapshot
                           "marlin.backend.composition"
                           '("marlin.profile.backend-unified")
                           gov-ok taint-bad fix-ok temporal-ok))
             ;; Clean proof stack -> synthesizes proof-bound CedarAuthoritySnapshot including
             ;; dynamically derived ASCENT `forbid` rules for transitive revoked agents!
             (ok-snap (synthesize-marlin-cedar-authority-snapshot
                       "marlin.backend.composition"
                       '("marlin.profile.backend-unified")
                       gov-ok taint-ok fix-ok temporal-ok
                       revocation-epoch: 2)))
        (check (.ref denied-snap 'admitted?) => #f)
        (check (.ref denied-snap 'rejection-reasons) => '(ascent-camel-taint-alert))
        (check (.ref ok-snap 'admitted?) => #t)
        (check (.ref ok-snap 'runtime-executed?) => #f)
        ;; Verify dynamically synthesized Cedar forbid policy for transitive revoked child agent!
        (let* ((policy-ids (map (lambda (p) (.ref p 'identity)) (.ref ok-snap 'policies)))
               (rt-hash (.ref ok-snap 'runtime-projection)))
          (check (and (member "marlin.ascent.forbid.fenced.agent-revoked-child" policy-ids) #t) => #t)
          (check (hash-ref rt-hash "schema_id") => "agentic-ai-lab.cedar-authority-snapshot.v1")
          (check (hash-ref rt-hash "revocation_epoch") => 2))
        ;; Strict one-shot AuthorizedEffectToken mediation + replay refusal + CedarAuthorizationRequest!
        (let* ((root0 "0000000000000000000000000000000000000000000000000000000000000000")
               (obs1  "1111111111111111111111111111111111111111111111111111111111111111")
               (s0 (agentic-ai-lab-strict-mediation-state root0 0 '() 2))
               (bundle (mint-governed-effect-token
                        "tok-1" "nonce-1"
                        (.ref gov-ok 'assessment-digest)
                        (.ref taint-ok 'proof-digest)
                        (.ref fix-ok 'proof-digest)
                        (.ref temporal-ok 'proof-digest)
                        "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                        "attempt-1" 'tool-process "marlin-kernel" "sess-1" "lease-1" 2))
               (first-med (mediate-governed-effect-token s0 bundle 10 root0 obs1))
               (s1 (.ref first-med 'next-state))
               ;; Replaying the same one-shot token MUST be refused fail-closed (`token-reuse`)!
               (replay-med (mediate-governed-effect-token s1 bundle 11 (.ref s1 'execution-root) obs1))
               (cedar-req (make-marlin-cedar-effect-request
                           "Marlin::Actor::\"ops-human\""
                           "Marlin::Action::\"dispatch_tool\""
                           "Marlin::Resource::\"workspace\""
                           (.o step_up_fresh: #t scope: "channel")
                           first-med
                           (string->utf8 "tool-payload"))))
          (check (.ref (.ref first-med 'receipt) 'accepted?) => #t)
          (check (.ref (.ref first-med 'receipt) 'outcome) => 'committed)
          (check (.ref (.ref replay-med 'receipt) 'accepted?) => #f)
          (check (.ref (.ref replay-med 'receipt) 'code) => 'token-reuse)
          (check (hash-ref (.ref cedar-req 'runtime-projection) "schema_id")
                 => "agentic-ai-lab.cedar-authorization-request.v1"))))

    ;; -----------------------------------------------------------------------
    ;; 7. Live ASCENT Incremental Session Deletion / Premise Withdrawal
    ;; -----------------------------------------------------------------------
    (test-case "gerbil-ascent incremental session retraction revokes unsupported grants while preserving alternate paths"
      (let* ((initial '((human agent-a)
                        (agent-a tool-exec)
                        (human agent-backup)
                        (agent-backup safe-read)
                        (agent-a safe-read)))
             ;; Retract the hop (human -> agent-a):
             ;;   - (human -> tool-exec) loses its ONLY grounded path -> REVOKED!
             ;;   - (human -> safe-read) retains alternate path via agent-backup -> SURVIVES!
             (revised '((agent-a tool-exec)
                        (human agent-backup)
                        (agent-backup safe-read)
                        (agent-a safe-read)))
             (res (run-ascent-incremental-retraction-session initial revised)))
        (check (and (member '(human tool-exec) (.ref res 'revoked-reach)) #t) => #t)
        (check (and (member '(human safe-read) (.ref res 'surviving-reach)) #t) => #t)
        (check (not (member '(human tool-exec) (.ref res 'after-reach))) => #t)))

    ;; -----------------------------------------------------------------------
    ;; 8. Bi-Temporal Truth Maintenance Reverse-Dependency Blast-Radius Invalidation
    ;; -----------------------------------------------------------------------
    (test-case "bi-temporal truth maintenance computes exact reverse-dependency blast radius on retraction & valid-time shift"
      (let* ((revs
              (list
               (make-governance-evidence-revision
                "r-sess-1" "human-session-alice" 'assert #f 1 valid-start: 0 valid-end: 50)
               (make-governance-evidence-revision
                "r-chan-1" "channel-cleanliness" 'assert #f 1 valid-start: 0 valid-end: 50)
               (make-governance-evidence-revision
                "r-pub-1" "public-workspace-policy" 'assert #f 1 valid-start: 0 valid-end: 50)
               ;; At admission=2, human-session-alice is retracted!
               (make-governance-evidence-revision
                "r-sess-retract" "human-session-alice" 'retract "r-sess-1" 2)))
             (derivs
              '(("D-OBO-TICKET" "pol.obo" ("human-session-alice") ())
                ("D-TOOL-EXEC" "pol.tool" ("channel-cleanliness") ("D-OBO-TICKET"))
                ("D-PUBLIC-DOC" "pol.public" ("public-workspace-policy") ())))
             (tm (evaluate-bitemporal-truth-maintenance
                  "tm-gov-journal" revs derivs 1 2 10
                  revised-valid-at-pos: 60)))
        ;; Exact blast radius: D-OBO-TICKET & transitive child D-TOOL-EXEC invalidated, D-PUBLIC-DOC spared!
        (check (.ref tm 'retracted-subjects) => '("human-session-alice"))
        (check (.ref tm 'affected-conclusions) => '("D-OBO-TICKET" "D-TOOL-EXEC"))
        ;; Shifting valid-at triggers full valid-time reprojection across all 3 derivations!
        (check (.ref (.ref tm 'reprojection-plan) 'trigger) => 'valid-time-reprojection)
        (check (length (.ref (.ref tm 'reprojection-plan) 'affected-conclusion-identities)) => 3)))

    ;; -----------------------------------------------------------------------
    ;; 9. Zero-JSON 14-Slot Rooted Gambit Vector Projection (Scheme -> Rust C-ABI)
    ;; -----------------------------------------------------------------------
    (test-case "zero-JSON 14-slot rooted governance vector projects taint, retraction, and Cedar policies"
      (let* ((req-blocked
              (vector
               #(#(alice agent-sub))                    ;; [0] delegation_edges
               #(alice)                                 ;; [1] revoked_actors
               #(#(chat-ext shared-ctx) #(shared-ctx os-shell)) ;; [2] taint_edges
               #(chat-ext)                              ;; [3] taint_sources
               #(os-shell)                              ;; [4] taint_sinks
               #()                                      ;; [5] taint_sanitizers
               #() #() #() #()                          ;; [6..9] context
               #() #()                                  ;; [10..11] breakers
               #("human-session-alice")                 ;; [12] retracted_subjects
               2                                        ;; [13] revocation_epoch
               1))                                      ;; [14] policy_revision
             (out-blocked (marlin-higher-order-governance-project-vector req-blocked))
             (req-admitted
              (vector
               #(#(alice agent-sub))
               #(alice)
               #(#(chat-ext human-gate) #(human-gate os-shell))
               #(chat-ext)
               #(os-shell)
               #(human-gate)
               #() #() #() #()
               #() #()
               #()
               3
               1))
             (out-admitted (marlin-higher-order-governance-project-vector req-admitted)))
        (check (vector-length out-blocked) => 14)
        (check (vector-ref out-blocked 0) => 1)
        (check (vector-ref out-blocked 1) => #f)
        (check (and (member "ascent-camel-taint-alert" (vector->list (vector-ref out-blocked 9))) #t) => #t)
        (check (and (member "bitemporal-premise-retracted" (vector->list (vector-ref out-blocked 9))) #t) => #t)
        (check (vector->list (vector-ref out-blocked 12)) => '("D-OBO-TICKET" "D-TOOL-EXEC"))
        ;; Admitted vector passes while still exporting ASCENT-derived fenced agent-sub & Cedar forbid!
        (check (vector-ref out-admitted 1) => #t)
        (check (vector-ref out-admitted 2) => 3)
        (check (and (member "agent-sub" (vector->list (vector-ref out-admitted 10))) #t) => #t)
        (check (> (vector-length (vector-ref out-admitted 13)) 2) => #t)))))
