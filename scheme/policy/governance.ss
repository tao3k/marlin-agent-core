;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Higher-Order AI Governance Plane amplifying `agentic-ai-lab`'s native
;;; Governance, ASCENT Datalog/Higher-Order Fixpoint, Bi-Temporal Causality,
;;; Atomic POO composition, One-Shot AuthorizedEffectToken Strict Mediation,
;;; and Proof-Bound Cedar Authority Snapshot synthesis.
;;;
;;; Architectural invariants:
;;;   1. Pure functional POO values only; zero side effects or runtime IO.
;;;   2. Assessment is NEVER authority: only a resolved POO GovernanceAssessment,
;;;      clean ASCENT taint/delegation fixpoint, replay-valid Bi-Temporal Causal
;;;      Cut, and Strict-Mediated one-shot token may synthesize a Cedar
;;;      AuthoritySnapshot handoff for the Rust execution boundary.
;;;   3. Atomic POO composition via prototype inheritance (`.o`, `:: @`, `.cc`)
;;;      and open `.defgeneric` dispatch keeps threat models and policy layers
;;;      extensible without a central switch table.

(import (only-in :clan/poo/object .cc .def .get .o .ref .slot? object?)
        (only-in :clan/poo/type Type. validate)
        (only-in :clan/poo/types/object-contract ObjectContract.)
        (only-in :std/crypto/digest sha256)
        (only-in :std/encoding/hex hex-encode)
        (only-in :std/list/list append-map delete-duplicates/hash filter filter-map)
        ;; 1. agentic-ai-lab native Governance POO prototypes & one-shot strict mediation
        (only-in :tao3k/agentic-ai-lab/modules/governance/interface
                 AgenticAILabGovernanceProfile.
                 AgenticAILabGovernanceThreatModel.
                 agentic-ai-lab-governance-source
                 agentic-ai-lab-governance-precondition
                 agentic-ai-lab-governance-threat
                 agentic-ai-lab-governance-threat-model
                 agentic-ai-lab-governance-evaluate)
        (only-in :tao3k/agentic-ai-lab/modules/governance/policy/authorized-effect-token
                 agentic-ai-lab-semantic-root
                 agentic-ai-lab-effect-binding
                 agentic-ai-lab-token-validity
                 agentic-ai-lab-authorized-effect-token
                 agentic-ai-lab-token-validation-context)
        (only-in :tao3k/agentic-ai-lab/modules/governance/policy/strict-mediation
                 agentic-ai-lab-strict-mediation-state
                 agentic-ai-lab-strict-mediate
                 agentic-ai-lab-strict-mediation-result-state
                 agentic-ai-lab-strict-mediation-result-receipt)
        ;; 2. gerbil-ascent higher-order relational fixpoint + stratified taint Datalog
        (only-in :gerbil-ascent/program/higher-order
                 relational-relation-type
                 relational-arrow-type
                 relational-typed-source
                 relational-typed-function
                 relational-typed-apply
                 relational-typed-union
                 relational-typed-join
                 relational-typed-project
                 relational-typed-fix
                 relational-typed-compile)
        (only-in :gerbil-ascent/program/func
                 gerbil-ascent-program
                 gerbil-ascent-relation
                 gerbil-ascent-rule
                 gerbil-ascent-atom
                 gerbil-ascent-negation
                 gerbil-ascent-variable)
        (only-in :gerbil-ascent/program/scheme-language
                 relational-admit
                 relational-solve
                 relational-query-name)
        (only-in :gerbil-ascent/program/session
                 gerbil-ascent-open-session
                 gerbil-ascent-session-run
                 gerbil-ascent-session-replace-source!)
        (only-in :gerbil-ascent/applications/taint-paths
                 gerbil-ascent-taint-path-program)
        ;; 3. agentic-ai-lab Bi-Temporal Causality + Truth Maintenance + gerbil-ascent temporal lens
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/objects
                 agentic-ai-lab-causal-event
                 agentic-ai-lab-temporal-observation)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/funs
                 agentic-ai-lab-causal-event-graph
                 agentic-ai-lab-causal-cut)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/time/objects
                 agentic-ai-lab-temporal-instant
                 agentic-ai-lab-temporal-interval)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/revisions/objects
                 agentic-ai-lab-temporal-evidence-revision)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/revisions/funs
                 agentic-ai-lab-temporal-evidence-journal
                 agentic-ai-lab-temporal-evidence-as-of)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/truth-maintenance/objects
                 agentic-ai-lab-temporal-derivation)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/truth-maintenance/funs
                 agentic-ai-lab-temporal-dependency-index
                 agentic-ai-lab-temporal-reverse-dependency-plan-from-cuts
                 agentic-ai-lab-temporal-reverse-dependency-plan-from-projections)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/ascent-exchange
                 agentic-ai-lab-ascent-event-binding
                 agentic-ai-lab-ascent-exchange
                 agentic-ai-lab-ascent-exchange-solve
                 agentic-ai-lab-ascent-exchange-verify)
        (only-in :gerbil-ascent/temporal/lens
                 temporal-status
                 temporal-rows
                 temporal-receipt)
        ;; 4. agentic-ai-lab Cedar Provider POO values & runtime boundary projection
        (only-in :tao3k/agentic-ai-lab/modules/authorization/providers/cedar/objects
                 agentic-ai-lab-cedar-policy
                 agentic-ai-lab-cedar-schema
                 agentic-ai-lab-cedar-entities
                 agentic-ai-lab-cedar-proof-binding
                 agentic-ai-lab-cedar-authority-context
                 agentic-ai-lab-cedar-runtime-capability
                 agentic-ai-lab-cedar-authority-snapshot
                 agentic-ai-lab-cedar-runtime-handoff
                 agentic-ai-lab-cedar-authorization-request
                 agentic-ai-lab-cedar-authority-snapshot->runtime
                 agentic-ai-lab-cedar-authorization-request->runtime))

(export ;; Layer 1: Atomic POO Governance Profiles & Combinators
        make-marlin-governance-profile
        compose-governance-profiles
        evaluate-marlin-governance-profile
        ;; Layer 2: ASCENT Higher-Order Fixpoint, Stratified Taint & Incremental Retraction
        solve-ascent-transitive-closure
        solve-ascent-camel-taint-paths
        solve-ascent-delegation-and-scope-governance
        run-ascent-incremental-retraction-session
        ;; Layer 3: Bi-Temporal Causality Cuts & Truth Maintenance Reverse-Dependency Blast Radius
        make-temporal-governance-event
        evaluate-temporal-causal-governance
        verify-temporal-causal-governance
        make-governance-evidence-revision
        evaluate-bitemporal-truth-maintenance
        ;; Layer 4: Strict One-Shot AuthorizedEffectToken Mediation
        mint-governed-effect-token
        mediate-governed-effect-token
        ;; Layer 5: Higher-Order Cedar Synthesis & Proof-Bound Runtime Snapshot
        synthesize-cedar-policies-from-ascent
        synthesize-marlin-cedar-authority-snapshot
        make-marlin-cedar-effect-request
        ;; Layer 6: Zero-JSON Rooted Gambit Heap Vector Projection (Scheme -> Rust C-ABI)
        marlin-governance-project-vector
        ;; Contract-validated POO Governance Policy Bundle
        GovernancePolicyContract
        GovernancePolicy)

;; ===========================================================================
;; Shared canonical content-digest utility (SHA-256, `sha256:<64-hex>`)
;; ===========================================================================

(def (sha256-digest value)
  (string-append
   "sha256:"
   (hex-encode
    (sha256
     (string->utf8
      (call-with-output-string "" (lambda (port) (write value port))))))))

;; ===========================================================================
;; Layer 1: Atomic POO Governance Profiles (extending agentic-ai-lab/governance)
;; ===========================================================================

;;; Builds a contract-validated `AgenticAILabGovernanceProfile` specialized via
;;; gerbil-poo prototype inheritance (`:: @ AgenticAILabGovernanceProfile.`).
(def (make-marlin-governance-profile profile-id rev owner-name
                                     threat-model-id threat-list
                                     source-list: (source-list '())
                                     ontology-obj: (ontology-obj (.o))
                                     policies-obj: (policies-obj (.o)))
  (let ((tm (agentic-ai-lab-governance-threat-model threat-model-id threat-list)))
    (.o (:: @ AgenticAILabGovernanceProfile.)
        identity: profile-id
        revision: rev
        owner: owner-name
        ontology: ontology-obj
        policies: policies-obj
        source-assets: source-list
        threat-model: tm)))

;;; Higher-order atomic POO combinator: composes multiple independent
;;; `AgenticAILabGovernanceProfile`s into one unified Profile.
;;; Monotonicity invariant: every threat and source asset from every child
;;; profile is preserved, so composing profiles can NEVER silence an exposed
;;; high/critical threat.
(def (compose-governance-profiles composed-id composed-rev composed-owner profiles)
  (when (null? profiles)
    (error "compose-governance-profiles requires at least one governance profile"))
  (let* ((all-threats
          (append-map (lambda (p) (.ref (.ref p 'threat-model) 'threats)) profiles))
         (all-sources
          (append-map (lambda (p) (.ref p 'source-assets)) profiles))
         (member-ids
          (map (lambda (p) (.ref p 'identity)) profiles))
         (composed-model-id (string-append composed-id "/threat-model")))
    (make-marlin-governance-profile
     composed-id composed-rev composed-owner
     composed-model-id all-threats
     source-list: all-sources
     ontology-obj: (.o composed-from: member-ids)
     policies-obj: (.o composition-mode: 'monotonic-conjunction))))

;;; Evaluates a Profile via `agentic-ai-lab-governance-evaluate` and wraps the
;;; assessment with a canonical `sha256:...` digest for downstream Cedar proof binding.
(def (evaluate-marlin-governance-profile profile context-obj)
  (let* ((assessment-val (agentic-ai-lab-governance-evaluate profile context-obj))
         (unresolved-val (.ref assessment-val 'unresolved-threats))
         (ready? (.ref assessment-val 'handoff-ready?))
         (digest-val (sha256-digest
                      (list 'marlin.governance-assessment.v1
                            (.ref assessment-val 'profile-identity)
                            (.ref assessment-val 'model-identity)
                            unresolved-val
                            ready?))))
    (.o kind: 'marlin.governance.assessment-receipt.v1
        assessment: assessment-val
        handoff-ready?: ready?
        unresolved-threats: unresolved-val
        assessment-digest: digest-val
        runtime-executed?: #f)))

;; ===========================================================================
;; Layer 2: ASCENT Higher-Order Fixpoint & Stratified Datalog Governance
;; ===========================================================================

;;; True Higher-Order Relational Policy Combinator powered by
;;; `:gerbil-ascent/program/higher-order`:
;;;   closure-builder : (Rel(2) -> Rel(2)) -> (Rel(2) -> Rel(2))
;;; Compiles a typed higher-order lambda + `relational-typed-fix` into a finite
;;; positive ASCENT Datalog program and solves exact transitive closure over
;;; `domain-atoms` (e.g. arbitrary-depth OBO delegation chains, cross-scope
;;; data flows, or cross-tenant trust graphs).
(def (solve-ascent-transitive-closure domain-atoms edge-pairs
                                            input-budget: (input-budget 128)
                                            derived-budget: (derived-budget 1024)
                                            output-budget: (output-budget 2048))
  (let* ((r2 (relational-relation-type 2 domain-atoms))
         (rr (relational-arrow-type r2 r2))
         (closure-builder
          (relational-typed-function
           rr
           (lambda (step-fn)
             (relational-typed-function
              r2
              (lambda (seed)
                (relational-typed-fix
                 (relational-typed-function
                  r2
                  (lambda (acc)
                    (relational-typed-union
                     seed
                     (relational-typed-apply step-fn acc))))))))))
         (edges-term (relational-typed-source 'gov_edges r2 edge-pairs))
         (step-term
          (relational-typed-function
           r2
           (lambda (paths)
             (relational-typed-project
              (relational-typed-join paths edges-term 1 0)
              '(0 3)))))
         (applied-term
          (relational-typed-apply
           (relational-typed-apply closure-builder step-term)
           edges-term)))
    (let-values (((program output-rel)
                  (relational-typed-compile
                   applied-term input-budget derived-budget output-budget)))
      (let* ((solved (relational-solve (relational-admit program)))
             (rows (relational-query-name solved output-rel))
             (digest-val (sha256-digest
                          (list 'marlin.ascent.transitive-closure.v1
                                domain-atoms edge-pairs rows))))
        (.o kind: 'marlin.ascent.transitive-closure-receipt.v1
            closure-rows: rows
            output-relation: output-rel
            proof-digest: digest-val)))))

;;; Stratified Datalog CaMeL Taint-Path & Declassification Engine built directly
;;; on `:gerbil-ascent/applications/taint-paths`:
;;;   - `edges`:      (list (list from to) ...)    — multi-hop data/context/prompt flows
;;;   - `sources`:    (list (list node) ...)       — untrusted chatroom/external/confidential roots
;;;   - `sinks`:      (list (list node) ...)       — OS process / disk write / external egress sinks
;;;   - `sanitizers`: (list (list node) ...)       — verified human gates / explicit declassification nodes
;;; Returns exact `taint_path` & `taint_alert` rows plus fail-closed `clean?` verdict.
(def (solve-ascent-camel-taint-paths edges sources sinks sanitizers
                                     input-budget: (input-budget 2048)
                                     fact-budget: (fact-budget 4096)
                                     output-budget: (output-budget 8192))
  (let* ((prog (gerbil-ascent-taint-path-program
                edges sources sinks sanitizers
                input-budget fact-budget output-budget))
         (solved (relational-solve (relational-admit prog)))
         (paths (relational-query-name solved 'taint_path))
         (alerts (relational-query-name solved 'taint_alert))
         (clean-flag (null? alerts))
         (digest-val (sha256-digest
                      (list 'marlin.ascent.camel-taint.v1
                            edges sources sinks sanitizers alerts))))
    (.o kind: 'marlin.ascent.camel-taint-receipt.v1
        clean?: clean-flag
        taint-paths: paths
        taint-alerts: alerts
        proof-digest: digest-val)))

;;; Multi-Hop Stratified Datalog Governance Fixpoint across:
;;;   - Arbitrary-depth OBO delegation chains (`delegates(parent, child)`)
;;;   - Revoked roots (`revoked_actor(actor)`) propagating transitively to ALL
;;;     downstream delegated agents (`fenced_actor(actor)`)
;;;   - Shared context entries (`context_entry(entry, scope)`) flowing along
;;;     prompt/context edges (`context_flow(from, to)`) into target scopes
;;;     (`target_scope(node, scope)`) where `narrower_scope(entry_scope, target_scope)`
;;;     triggers `scope_exfiltration_alert(entry, node)` UNLESS blocked by an
;;;     explicit owner declassification (`declassified_entry(entry)`).
;;;   - Active execution breaker trip (`breaker_tripped(breaker)`) fencing every
;;;     bound tool execution (`breaker_blocked_tool(tool)`).
(def (solve-ascent-delegation-and-scope-governance
      delegation-edges: (delegation-edges '())
      revoked-actors: (revoked-actors '())
      context-entries: (context-entries '())
      context-flows: (context-flows '())
      target-scopes: (target-scopes '())
      declassified-entries: (declassified-entries '())
      tripped-breakers: (tripped-breakers '())
      breaker-tools: (breaker-tools '()))
  (let ((p (gerbil-ascent-variable 'p))
        (c (gerbil-ascent-variable 'c))
        (x (gerbil-ascent-variable 'x))
        (y (gerbil-ascent-variable 'y))
        (e (gerbil-ascent-variable 'e))
        (s1 (gerbil-ascent-variable 's1))
        (s2 (gerbil-ascent-variable 's2))
        (b (gerbil-ascent-variable 'b))
        (t (gerbil-ascent-variable 't)))
    (def (a name . terms) (gerbil-ascent-atom name terms))
    ;; Strict 5-scope visibility order: private < direct-message < channel < workspace < external
    (def narrower-pairs
      '((private direct-message) (private channel) (private workspace) (private external)
        (direct-message channel) (direct-message workspace) (direct-message external)
        (channel workspace) (channel external)
        (workspace external)))
    (let* ((prog
            (gerbil-ascent-program
             (list
              (gerbil-ascent-relation 'delegates 2 delegation-edges)
              (gerbil-ascent-relation 'revoked_actor 1 revoked-actors)
              (gerbil-ascent-relation 'fenced_actor 1 '())
              (gerbil-ascent-relation 'context_entry 2 context-entries)
              (gerbil-ascent-relation 'context_flow 2 context-flows)
              (gerbil-ascent-relation 'entry_reaches 2 '())
              (gerbil-ascent-relation 'target_scope 2 target-scopes)
              (gerbil-ascent-relation 'narrower_scope 2 narrower-pairs)
              (gerbil-ascent-relation 'declassified_entry 1 declassified-entries)
              (gerbil-ascent-relation 'scope_exfiltration_alert 2 '())
              (gerbil-ascent-relation 'breaker_tripped 1 tripped-breakers)
              (gerbil-ascent-relation 'breaker_tool 2 breaker-tools)
              (gerbil-ascent-relation 'breaker_blocked_tool 1 '()))
             (list
              ;; Rule 1: Transitive revocation propagation along OBO delegation chains
              (gerbil-ascent-rule (list (a 'fenced_actor p))
                                  (list (a 'revoked_actor p)))
              (gerbil-ascent-rule (list (a 'fenced_actor c))
                                  (list (a 'fenced_actor p) (a 'delegates p c)))
              ;; Rule 2: Multi-hop context reachability & stratified anti-exfiltration alert
              (gerbil-ascent-rule (list (a 'entry_reaches e e))
                                  (list (a 'context_entry e s1)))
              (gerbil-ascent-rule (list (a 'entry_reaches e y))
                                  (list (a 'entry_reaches e x) (a 'context_flow x y)))
              (gerbil-ascent-rule (list (a 'scope_exfiltration_alert e y))
                                  (list (a 'context_entry e s1)
                                        (a 'entry_reaches e y)
                                        (a 'target_scope y s2)
                                        (a 'narrower_scope s1 s2)
                                        (gerbil-ascent-negation 'declassified_entry (list e))))
              ;; Rule 3: Tripped execution circuit breaker fences all bound tools
              (gerbil-ascent-rule (list (a 'breaker_blocked_tool t))
                                  (list (a 'breaker_tripped b) (a 'breaker_tool b t))))
             4096 8192 16384))
           (solved (relational-solve (relational-admit prog)))
           (fenced (relational-query-name solved 'fenced_actor))
           (exfil-alerts (relational-query-name solved 'scope_exfiltration_alert))
           (blocked-tools (relational-query-name solved 'breaker_blocked_tool))
           (clean-flag (and (null? exfil-alerts) (null? blocked-tools)))
           (digest-val (sha256-digest
                        (list 'marlin.ascent.governance-fixpoint.v1
                              fenced exfil-alerts blocked-tools))))
      (.o kind: 'marlin.ascent.governance-fixpoint-receipt.v1
          clean?: clean-flag
          fenced-actors: fenced
          scope-exfiltration-alerts: exfil-alerts
          breaker-blocked-tools: blocked-tools
          proof-digest: digest-val))))

;;; Layer 2B: Live ASCENT Incremental Session Deletion / Premise Withdrawal.
;;; Opens one stateful ASCENT session over `initial-edges` -> solves cut 1 ->
;;; incrementally replaces the edge relation with `revised-edges` via
;;; `gerbil-ascent-session-replace-source!` -> solves cut 2.
;;; Computes exact `revoked-reach` (pairs valid in cut 1 that lost all grounded
;;; support in cut 2) and `surviving-reach` (pairs retaining an independent path).
(def (run-ascent-incremental-retraction-session initial-edges revised-edges)
  (let ((x (gerbil-ascent-variable 'x))
        (y (gerbil-ascent-variable 'y))
        (z (gerbil-ascent-variable 'z)))
    (def (a name . terms) (gerbil-ascent-atom name terms))
    (let* ((prog
            (gerbil-ascent-program
             (list (gerbil-ascent-relation 'auth_edge 2 initial-edges)
                   (gerbil-ascent-relation 'auth_reach 2 '()))
             (list (gerbil-ascent-rule (list (a 'auth_reach x y))
                                       (list (a 'auth_edge x y)))
                   (gerbil-ascent-rule (list (a 'auth_reach x z))
                                       (list (a 'auth_reach x y) (a 'auth_edge y z))))
             1024 4096 8192))
           (session (gerbil-ascent-open-session prog))
           (res1 (gerbil-ascent-session-run session))
           (before-rows ((.ref res1 'rows-of) 'auth_reach))
           (_ (gerbil-ascent-session-replace-source! session 'auth_edge revised-edges))
           (res2 (gerbil-ascent-session-run session))
           (after-rows ((.ref res2 'rows-of) 'auth_reach))
           (revoked-rows (filter (lambda (r) (not (member r after-rows))) before-rows))
           (surviving-rows (filter (lambda (r) (and (member r after-rows) #t)) before-rows))
           (digest-val (sha256-digest
                        (list 'marlin.ascent.incremental-retraction.v1
                              before-rows after-rows revoked-rows))))
      (.o kind: 'marlin.ascent.incremental-retraction-receipt.v1
          before-reach: before-rows
          after-reach: after-rows
          revoked-reach: revoked-rows
          surviving-reach: surviving-rows
          proof-digest: digest-val))))

;; ===========================================================================
;; Layer 3: Bi-Temporal Causality Governance Cuts (valid-time x knowledge-time)
;; ===========================================================================

;;; Constructs one governed causal event with explicit logical position, parents,
;;; valid-time coordinate, and knowledge-time coordinate (never conflated!).
(def (make-temporal-governance-event ev-id rt-sym pos-val parent-ids
                                     valid-pos knowledge-pos
                                     subject: (subject-name "marlin-governance"))
  (.o kind: 'marlin.temporal-governance-event-spec.v1
      event-id: ev-id
      runtime-sym: rt-sym
      logical-pos: pos-val
      parents: parent-ids
      valid-coord: valid-pos
      knowledge-coord: knowledge-pos
      subject: subject-name))

(def (build-temporal-exchange event-specs as-of-pos valid-start valid-end knowledge-as-of horizon
                              subject-name: (subject-name "marlin-governance")
                              source-name: (source-name 'marlin-gov-ledger))
  (let* ((events
          (map (lambda (spec)
                 (let ((eid (.ref spec 'event-id))
                       (pos (.ref spec 'logical-pos))
                       (pars (.ref spec 'parents)))
                   (agentic-ai-lab-causal-event
                    eid subject-name 'observed
                    (agentic-ai-lab-temporal-observation
                     eid 'logical-version pos "marlin-governance-ledger")
                    eid pars 'observed #t)))
               event-specs))
         (graph (agentic-ai-lab-causal-event-graph subject-name events))
         (cut (agentic-ai-lab-causal-cut graph as-of-pos))
         (bindings
          (map (lambda (spec)
                 (let ((eid (.ref spec 'event-id))
                       (rsym (.ref spec 'runtime-sym))
                       (vc (.ref spec 'valid-coord))
                       (kc (.ref spec 'knowledge-coord)))
                   (agentic-ai-lab-ascent-event-binding
                    eid rsym
                    (agentic-ai-lab-temporal-instant
                     (string-append eid "-valid") "valid" vc "marlin-gov" 'observed)
                    (agentic-ai-lab-temporal-instant
                     (string-append eid "-known") "knowledge" kc "marlin-gov" 'observed))))
               event-specs)))
    (agentic-ai-lab-ascent-exchange
     graph cut bindings
     source-name 0 'marlin-clock "valid" "knowledge" 'marlin-gov-cut
     valid-start valid-end knowledge-as-of horizon #t)))

;;; Solves bi-temporal causal reachability from `root-event-id` through
;;; `agentic-ai-lab-ascent-exchange-solve` + `gerbil-ascent/temporal/lens`,
;;; producing a tamper-evident ASCENT causal receipt and verifying whether any
;;; `forbidden-ancestor-syms` (e.g. a revocation or poison event) appear in the
;;; active causal cut.
(def (evaluate-temporal-causal-governance event-specs root-ev-id
                                          as-of-pos valid-start valid-end
                                          knowledge-as-of horizon
                                          forbidden-event-syms: (forbidden-event-syms '()))
  (let* ((exchange-val (build-temporal-exchange
                        event-specs as-of-pos valid-start valid-end knowledge-as-of horizon))
         (answer-val (agentic-ai-lab-ascent-exchange-solve exchange-val root-ev-id))
         (status-val (temporal-status answer-val))
         (rows-val (temporal-rows answer-val))
         (receipt-val (temporal-receipt answer-val))
         (reached-nodes (delete-duplicates/hash (apply append rows-val)))
         (forbidden-val (filter (lambda (sym) (and (memq sym reached-nodes) #t))
                                forbidden-event-syms))
         (replay-val (agentic-ai-lab-ascent-exchange-verify exchange-val root-ev-id answer-val))
         (admitted-flag (and (eq? status-val 'complete)
                             (eq? replay-val 'valid)
                             (null? forbidden-val)))
         (digest-val (sha256-digest
                      (list 'marlin.temporal-causal-governance.v1
                            root-ev-id status-val rows-val forbidden-val replay-val))))
    (.o kind: 'marlin.temporal-causal-governance-receipt.v1
        admitted?: admitted-flag
        temporal-status: status-val
        replay-status: replay-val
        causal-rows: rows-val
        forbidden-hits: forbidden-val
        temporal-receipt: receipt-val
        exchange: exchange-val
        answer: answer-val
        root-event-id: root-ev-id
        proof-digest: digest-val)))

;;; Re-verifies a previously solved causal governance receipt against a
;;; candidate event history; returns `'valid` iff the bi-temporal valid/knowledge
;;; coordinates and causal ancestry match bit-for-bit, or `'invalid` on tampering.
(def (verify-temporal-causal-governance receipt candidate-event-specs
                                        as-of-pos valid-start valid-end
                                        knowledge-as-of horizon)
  (let ((candidate-exchange
         (build-temporal-exchange
          candidate-event-specs as-of-pos valid-start valid-end knowledge-as-of horizon)))
    (agentic-ai-lab-ascent-exchange-verify
     candidate-exchange
     (.ref receipt 'root-event-id)
     (.ref receipt 'answer))))

;;; Layer 3B: Bi-Temporal Evidence Journal & Truth Maintenance Reverse-Dependency
;;; Blast-Radius Invalidation (`agentic-ai-lab/modules/temporal-causality/{revisions,truth-maintenance}`).
;;;
;;; Separates admission-time (`as-of`) from validity interval (`valid-start..valid-end`):
;;;   - `'assert`:  introduces a governed premise valid over `[valid-start, valid-end]`
;;;   - `'correct`: supersedes a predecessor revision with updated content/interval
;;;   - `'retract`: withdraws a predecessor revision (valid-interval = #f, content = #f)
(def (make-governance-evidence-revision rev-id subject-id op-sym pred-id
                                        admission-pos
                                        valid-start: (valid-start 0)
                                        valid-end: (valid-end 100)
                                        content-id: (content-id #f))
  (let ((adm-inst (agentic-ai-lab-temporal-instant
                   (string-append rev-id "-adm") "admission" admission-pos
                   "marlin-gov-journal" 'observed))
        (valid-iv
         (if (eq? op-sym 'retract)
           #f
           (agentic-ai-lab-temporal-interval
            (string-append rev-id "-valid")
            (agentic-ai-lab-temporal-instant
             (string-append rev-id "-vs") "valid" valid-start "marlin-gov-journal" 'observed)
            (agentic-ai-lab-temporal-instant
             (string-append rev-id "-ve") "valid" valid-end "marlin-gov-journal" 'observed)
            #t #t))))
    (agentic-ai-lab-temporal-evidence-revision
     rev-id subject-id op-sym pred-id adm-inst valid-iv
     (if (eq? op-sym 'retract) #f (or content-id (string-append "content:" rev-id))))))

;;; Evaluates reverse-dependency invalidation over a bi-temporal evidence journal:
;;;   - `derivation-specs`: list of `(list deriv-id policy-id subject-premises conclusion-premises)`
;;;   - Computes `previous` snapshot at `previous-as-of` / `valid-at`
;;;   - Builds `agentic-ai-lab-temporal-dependency-index` pinned to `previous` cut & projection
;;;   - Computes `cut-plan` from `previous-as-of` -> `revised-as-of` (exact premise-delta blast radius!)
;;;   - Optionally computes `reprojection-plan` if `revised-valid-at` is provided (`'valid-time-reprojection`)
(def (evaluate-bitemporal-truth-maintenance
      journal-id revisions derivation-specs
      previous-as-of-pos revised-as-of-pos valid-at-pos
      revised-valid-at-pos: (revised-valid-at-pos #f))
  (let* ((journal (agentic-ai-lab-temporal-evidence-journal journal-id "admission" revisions))
         (prev-as-of (agentic-ai-lab-temporal-instant
                      "cut-prev" "admission" previous-as-of-pos "marlin-gov" 'observed))
         (rev-as-of (agentic-ai-lab-temporal-instant
                     "cut-rev" "admission" revised-as-of-pos "marlin-gov" 'observed))
         (val-inst (agentic-ai-lab-temporal-instant
                    "valid-at" "valid" valid-at-pos "marlin-gov" 'observed))
         (prev-snap (agentic-ai-lab-temporal-evidence-as-of journal prev-as-of val-inst))
         (rev-snap (agentic-ai-lab-temporal-evidence-as-of journal rev-as-of val-inst))
         (cut-d (.ref prev-snap 'cut-digest))
         (proj-d (.ref prev-snap 'projection-digest))
         (derivations
          (map (lambda (spec)
                 (agentic-ai-lab-temporal-derivation
                  (car spec) cut-d proj-d (cadr spec) (caddr spec) (cadddr spec)))
               derivation-specs))
         (dep-index (agentic-ai-lab-temporal-dependency-index
                     (string-append journal-id "/dep-index")
                     cut-d proj-d #t derivations))
         (cut-plan (agentic-ai-lab-temporal-reverse-dependency-plan-from-cuts
                    dep-index journal prev-as-of rev-as-of val-inst))
         (reproj-plan
          (and revised-valid-at-pos
               (let ((rev-val (agentic-ai-lab-temporal-instant
                               "valid-at-revised" "valid" revised-valid-at-pos
                               "marlin-gov" 'observed)))
                 (agentic-ai-lab-temporal-reverse-dependency-plan-from-projections
                  dep-index journal prev-as-of val-inst rev-val))))
         (affected-vals (.ref cut-plan 'affected-conclusion-identities))
         (retracted-vals (.ref rev-snap 'retracted-subject-identities))
         (conflicted-vals (.ref rev-snap 'conflicted-subject-identities))
         (outside-vals (.ref rev-snap 'outside-valid-subject-identities))
         (digest-val (sha256-digest
                      (list 'marlin.bitemporal-truth-maintenance.v1
                            journal-id cut-d (.ref rev-snap 'cut-digest)
                            affected-vals retracted-vals conflicted-vals outside-vals))))
    (.o kind: 'marlin.bitemporal-truth-maintenance-receipt.v1
        previous-snapshot: prev-snap
        revised-snapshot: rev-snap
        dependency-index: dep-index
        invalidation-plan: cut-plan
        reprojection-plan: reproj-plan
        affected-conclusions: affected-vals
        retracted-subjects: retracted-vals
        conflicted-subjects: conflicted-vals
        outside-valid-subjects: outside-vals
        invalidation-digest: digest-val
        runtime-executed?: #f)))

;; ===========================================================================
;; Layer 4: Strict One-Shot AuthorizedEffectToken Mediation
;; ===========================================================================

(def (mint-governed-effect-token token-id nonce-val
                                 bundle-digest policy-digest entity-digest
                                 decision-digest intent-digest
                                 attempt-id effect-kind runtime-id session-id
                                 lease-id revocation-epoch
                                 issued-at: (issued-at 1)
                                 not-before: (not-before 1)
                                 expiry: (expiry 1000))
  (let* ((root-val (agentic-ai-lab-semantic-root
                    bundle-digest policy-digest entity-digest decision-digest intent-digest))
         (binding-val (agentic-ai-lab-effect-binding
                       bundle-digest 1 policy-digest entity-digest decision-digest intent-digest
                       attempt-id effect-kind runtime-id session-id 1 1
                       "marlin-arena" 1 0 64 intent-digest lease-id))
         (validity-val (agentic-ai-lab-token-validity issued-at not-before expiry revocation-epoch))
         (token-val (agentic-ai-lab-authorized-effect-token
                     token-id nonce-val root-val binding-val validity-val 'strict '(ascent temporal cedar)
                     "marlin-governance-policy" "sig:poo-root")))
    (.o kind: 'marlin.governed-effect-bundle.v1
        semantic-root: root-val
        binding: binding-val
        token: token-val)))

(def (mediate-governed-effect-token state effect-bundle current-time expected-root observation)
  (let* ((token (.ref effect-bundle 'token))
         (ctx (agentic-ai-lab-token-validation-context
               (.ref effect-bundle 'semantic-root)
               (.ref effect-bundle 'binding)
               current-time
               (.ref state 'revocation-epoch)
               (.ref state 'consumed-nonces)))
         (res (agentic-ai-lab-strict-mediate state token ctx expected-root observation)))
    (.o kind: 'marlin.strict-mediation-outcome.v1
        next-state: (agentic-ai-lab-strict-mediation-result-state res)
        receipt: (agentic-ai-lab-strict-mediation-result-receipt res))))

;; ===========================================================================
;; Layer 5: Dynamic Cedar Synthesis & Proof-Bound Runtime Authority Snapshot
;; ===========================================================================

;;; Synthesizes concrete POO `CedarPolicy` artifacts directly from ASCENT
;;; fixpoint derivations (`fenced-actors` + `breaker-blocked-tools`) combined
;;; with Marlin's immutable baseline invariants (forbidding AI agents from
;;; breaker re-arm, scope promotion, and human gate resolution).
(def (synthesize-cedar-policies-from-ascent ascent-fixpoint-receipt)
  (let* ((fenced-rows (.ref ascent-fixpoint-receipt 'fenced-actors))
         (blocked-tool-rows (.ref ascent-fixpoint-receipt 'breaker-blocked-tools))
         (exfil-rows (.ref ascent-fixpoint-receipt 'scope-exfiltration-alerts))
         (baseline-policies
          (list
           (agentic-ai-lab-cedar-policy
            "marlin.forbid.agent-privileged-governance"
            (string-append
             "forbid (\n"
             "  principal in Marlin::Role::\"Agent\",\n"
             "  action in [\n"
             "    Marlin::Action::\"rearm_breaker\",\n"
             "    Marlin::Action::\"promote_context_scope\",\n"
             "    Marlin::Action::\"resolve_human_gate\",\n"
             "    Marlin::Action::\"unblock_matter\"\n"
             "  ],\n"
             "  resource\n"
             ");"))
           (agentic-ai-lab-cedar-policy
            "marlin.permit.verified-human-governance"
            (string-append
             "permit (\n"
             "  principal in Marlin::Role::\"VerifiedHuman\",\n"
             "  action in [\n"
             "    Marlin::Action::\"rearm_breaker\",\n"
             "    Marlin::Action::\"promote_context_scope\",\n"
             "    Marlin::Action::\"resolve_human_gate\"\n"
             "  ],\n"
             "  resource\n"
             ") when { context.step_up_fresh == true };"))))
         (ascent-fenced-policies
          (map (lambda (row)
                 (let ((actor-str (if (symbol? (car row))
                                    (symbol->string (car row))
                                    (car row))))
                   (agentic-ai-lab-cedar-policy
                    (string-append "marlin.ascent.forbid.fenced." actor-str)
                    (string-append
                     "forbid (principal == Marlin::Actor::\"" actor-str
                     "\", action, resource);"))))
               fenced-rows))
         (ascent-tool-policies
          (map (lambda (row)
                 (let ((tool-str (if (symbol? (car row))
                                   (symbol->string (car row))
                                   (car row))))
                   (agentic-ai-lab-cedar-policy
                    (string-append "marlin.ascent.forbid.breaker-tool." tool-str)
                    (string-append
                     "forbid (principal, action == Marlin::Action::\"" tool-str
                     "\", resource);"))))
               blocked-tool-rows))
         (ascent-exfil-policies
          (map (lambda (row)
                 (let ((entry-str (if (symbol? (car row))
                                    (symbol->string (car row))
                                    (car row)))
                       (target-str (if (symbol? (cadr row))
                                     (symbol->string (cadr row))
                                     (cadr row))))
                   (agentic-ai-lab-cedar-policy
                    (string-append "marlin.ascent.forbid.exfil." entry-str "." target-str)
                    (string-append
                     "forbid (principal, action, resource == Marlin::Resource::\"" target-str
                     "\");"))))
               exfil-rows))
         (ascent-scope-guard-policies
          (if (pair? exfil-rows)
            (list
             (agentic-ai-lab-cedar-policy
              "marlin.ascent.forbid.scope.external-unless-declassified"
              "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };"))
            '())))
    (append baseline-policies
            ascent-fenced-policies
            ascent-tool-policies
            ascent-exfil-policies
            ascent-scope-guard-policies)))

;;; Fail-closed higher-order synthesis gate: constructs a proof-bound
;;; `CedarAuthoritySnapshot` + its runtime boundary hash projection ONLY IF:
;;;   1. POO `AgenticAILabGovernanceAssessment` has zero unresolved blocking threats;
;;;   2. ASCENT CaMeL taint-path proof has zero un-sanitized alerts (`clean? == #t`);
;;;   3. ASCENT delegation/scope/breaker fixpoint has zero scope exfiltrations;
;;;   4. Bi-Temporal Causal Cut is complete, replay-valid, and free of forbidden ancestors.
(def (synthesize-marlin-cedar-authority-snapshot
      composition-id profile-ids
      governance-receipt taint-receipt fixpoint-receipt temporal-receipt
      revocation-epoch: (revocation-epoch 1)
      policy-revision: (policy-revision 1)
      generation: (generation 1)
      capabilities: (capabilities
                     (list (agentic-ai-lab-cedar-runtime-capability
                            "Marlin::Action::\"dispatch_tool\"" 1
                            source-admission-required?: #t))))
  (let* ((gov-ok? (.ref governance-receipt 'handoff-ready?))
         (taint-ok? (.ref taint-receipt 'clean?))
         (exfil-ok? (null? (.ref fixpoint-receipt 'scope-exfiltration-alerts)))
         (temporal-ok? (.ref temporal-receipt 'admitted?))
         (reasons
          (append
           (if gov-ok? '() '(unresolved-governance-threats))
           (if taint-ok? '() '(ascent-camel-taint-alert))
           (if exfil-ok? '() '(ascent-scope-exfiltration-alert))
           (if temporal-ok? '() '(temporal-causal-cut-rejected)))))
    (if (pair? reasons)
      (.o kind: 'marlin.cedar-authority-synthesis-receipt.v1
          admitted?: #f
          rejection-reasons: reasons
          snapshot: #f
          runtime-projection: #f
          runtime-executed?: #f)
      (let* ((policy-list (synthesize-cedar-policies-from-ascent fixpoint-receipt))
             (schema-doc (agentic-ai-lab-cedar-schema
                          "{\"Marlin\":{\"entityTypes\":{\"Actor\":{},\"Role\":{},\"Resource\":{}},\"actions\":{}}}"))
             (entities-doc (agentic-ai-lab-cedar-entities "[]"))
             (origin-digest (sha256-digest (list 'marlin.profile-origin profile-ids)))
             (profile-digest (sha256-digest (list 'marlin.profile-bundle
                                                  (.ref governance-receipt 'assessment-digest)
                                                  (.ref taint-receipt 'proof-digest))))
             (indep-digest (sha256-digest (list 'marlin.independent-proofs
                                                (.ref fixpoint-receipt 'proof-digest)
                                                (.ref temporal-receipt 'proof-digest))))
             (cap-digest (sha256-digest (list 'marlin.capability-contract
                                              (map (lambda (c) (.ref c 'action)) capabilities))))
             (subject-digest (.ref temporal-receipt 'proof-digest))
             (proof-val
              (agentic-ai-lab-cedar-proof-binding
               composition-id
               profile-ids
               origin-digest
               profile-digest
               indep-digest
               cap-digest
               (.ref governance-receipt 'assessment-digest)
               subject-digest
               (list "poo-governance-assessment"
                     "gerbil-ascent-camel-taint"
                     "gerbil-ascent-delegation-scope-fixpoint"
                     "agentic-ai-lab-bitemporal-causality")))
             (auth-ctx
              (agentic-ai-lab-cedar-authority-context
               "marlin.backend.authority"
               "marlin.kernel.side-effects"
               generation
               profile-digest
               1
               policy-revision
               revocation-epoch))
             (snapshot-val
              (agentic-ai-lab-cedar-authority-snapshot
               auth-ctx proof-val policy-list schema-doc entities-doc capabilities))
             (runtime-proj (agentic-ai-lab-cedar-authority-snapshot->runtime snapshot-val)))
        (.o kind: 'marlin.cedar-authority-synthesis-receipt.v1
            admitted?: #t
            rejection-reasons: '()
            snapshot: snapshot-val
            policies: policy-list
            proof-binding: proof-val
            runtime-projection: runtime-proj
            runtime-executed?: #f)))))

(def (make-marlin-cedar-effect-request principal action resource context-obj
                                       mediation-outcome payload-bytes
                                       sequence: (seq 1))
  (let* ((receipt (.ref mediation-outcome 'receipt))
         (accepted? (.ref receipt 'accepted?)))
    (unless accepted?
      (error "cannot project CedarAuthorizationRequest from rejected strict mediation receipt"
             (.ref receipt 'code)))
    (let* ((before-root (string-append "sha256:" (.ref receipt 'before-root)))
           (after-root (string-append "sha256:" (.ref receipt 'after-root)))
           (consumption (.ref receipt 'consumption))
           (sem-root (string-append "sha256:" (.ref (.ref consumption 'semantic-root) 'digest)))
           (obs-digest (string-append "sha256:" (.ref consumption 'observation-digest)))
           (intent-digest (sha256-digest (list principal action resource seq)))
           (handoff (agentic-ai-lab-cedar-runtime-handoff
                     seq payload-bytes sem-root before-root after-root obs-digest))
           (req (agentic-ai-lab-cedar-authorization-request
                 principal action resource context-obj intent-digest handoff)))
      (.o kind: 'marlin.cedar-effect-request-receipt.v1
          request: req
          runtime-projection: (agentic-ai-lab-cedar-authorization-request->runtime req)
          runtime-executed?: #f))))

;; ===========================================================================
;; Layer 6: Zero-JSON Rooted Gambit Heap Vector Projection (Scheme -> Rust C-ABI)
;; ===========================================================================

(def (scalar->sym v)
  (cond ((symbol? v) v)
        ((string? v) (string->symbol v))
        (else (error "expected symbol or string in governance vector" v))))

(def (scalar->str v)
  (cond ((string? v) v)
        ((symbol? v) (symbol->string v))
        (else (error "expected string or symbol in governance vector" v))))

(def (vec->pair-rows v)
  (map (lambda (pair-vec)
         (list (scalar->sym (vector-ref pair-vec 0))
               (scalar->sym (vector-ref pair-vec 1))))
       (vector->list v)))

(def (vec->singleton-rows v)
  (map (lambda (item) (list (scalar->sym item)))
       (vector->list v)))

(def (vec->derivation-specs v)
  (map (lambda (spec-vec)
         (unless (and (vector? spec-vec) (= (vector-length spec-vec) 4))
           (error "derivation spec must be a 4-slot vector #(id policy premises parents)" spec-vec))
         (list (scalar->str (vector-ref spec-vec 0))
               (scalar->str (vector-ref spec-vec 1))
               (map scalar->str (vector->list (vector-ref spec-vec 2)))
               (map scalar->str (vector->list (vector-ref spec-vec 3)))))
       (vector->list v)))

;;; Evaluates the entire 5-layer Higher-Order Governance Plane from a single
;;; zero-JSON 15-or-16-slot rooted Gambit Scheme input vector and returns a canonical
;;; 14-slot rooted Scheme output vector consumed directly by Rust `marlin-gerbil-scheme`
;;; and `marlin-agent-kernel::governance_admission`:
;;;
;;; Input vector (15 or 16 slots):
;;;   [0]  delegation_edges      : #(#(parent child) ...)
;;;   [1]  revoked_actors        : #(actor ...)
;;;   [2]  taint_edges           : #(#(from to) ...)
;;;   [3]  taint_sources         : #(node ...)
;;;   [4]  taint_sinks           : #(node ...)
;;;   [5]  taint_sanitizers      : #(node ...)
;;;   [6]  context_entries       : #(#(entry scope) ...)
;;;   [7]  context_flows         : #(#(from to) ...)
;;;   [8]  target_scopes         : #(#(node scope) ...)
;;;   [9]  declassified_entries  : #(entry ...)
;;;   [10] tripped_breakers      : #(breaker ...)
;;;   [11] breaker_tools         : #(#(breaker tool) ...)
;;;   [12] retracted_subjects    : #(subject-str ...)  (triggers Bi-Temporal Truth Maintenance!)
;;;   [13] revocation_epoch      : fixnum (>= 1)
;;;   [14] policy_revision       : fixnum (>= 1)
;;;   [15] derivation_specs (opt): #(#(deriv-id policy-id #(premise ...) #(parent-deriv ...)) ...)
;;;
;;; Output vector (14 slots):
;;;   [0]  schema_version        : 1
;;;   [1]  admitted?             : bool
;;;   [2]  revocation_epoch      : fixnum
;;;   [3]  policy_revision       : fixnum
;;;   [4]  assessment_digest     : "sha256:..."
;;;   [5]  taint_proof_digest    : "sha256:..."
;;;   [6]  fixpoint_proof_digest : "sha256:..."
;;;   [7]  temporal_proof_digest : "sha256:..."
;;;   [8]  invalidation_digest   : "sha256:..."
;;;   [9]  rejection_reasons     : #(reason-str ...)
;;;   [10] fenced_actors         : #(actor-str ...)
;;;   [11] blocked_tools         : #(tool-str ...)
;;;   [12] retracted_conclusions : #(derivation-id-str ...)
;;;   [13] cedar_policies        : #(#(policy-id-str cedar-source-str) ...)
(def (marlin-governance-project-vector req-vec)
  (unless (and (vector? req-vec)
               (or (= (vector-length req-vec) 15)
                   (= (vector-length req-vec) 16)))
    (error "governance projection request must be a 15-or-16-slot rooted vector" req-vec))
  (let* ((del-edges (vec->pair-rows (vector-ref req-vec 0)))
         (rev-actors (vec->singleton-rows (vector-ref req-vec 1)))
         (taint-edges (vec->pair-rows (vector-ref req-vec 2)))
         (taint-sources (vec->singleton-rows (vector-ref req-vec 3)))
         (taint-sinks (vec->singleton-rows (vector-ref req-vec 4)))
         (taint-sanitizers (vec->singleton-rows (vector-ref req-vec 5)))
         (ctx-entries (vec->pair-rows (vector-ref req-vec 6)))
         (ctx-flows (vec->pair-rows (vector-ref req-vec 7)))
         (tgt-scopes (vec->pair-rows (vector-ref req-vec 8)))
         (declass-entries (vec->singleton-rows (vector-ref req-vec 9)))
         (tripped-brks (vec->singleton-rows (vector-ref req-vec 10)))
         (brk-tools (vec->pair-rows (vector-ref req-vec 11)))
         (retracted-subs (map scalar->str (vector->list (vector-ref req-vec 12))))
         (rev-epoch (vector-ref req-vec 13))
         (pol-rev (vector-ref req-vec 14))
         (custom-derivs
          (if (= (vector-length req-vec) 16)
            (vec->derivation-specs (vector-ref req-vec 15))
            '()))
         ;; 1. Atomic POO Governance Profile assessment
         (base-profile
          (make-marlin-governance-profile
           "marlin.profile.governance-vm" "1.0.0" "tao3k-security"
           "tm.governance-vm"
           (list (agentic-ai-lab-governance-threat
                  "marlin.breaker.latch" 'critical 'mitigated
                  (list (agentic-ai-lab-governance-precondition
                         "marlin.breaker.latch.pre" #t "verified-by-quint"))
                  mitigations: '("rust-execution-circuit-breaker")))))
         (gov-receipt (evaluate-marlin-governance-profile base-profile (.o scene: 'vm-bridge)))
         ;; 2. ASCENT CaMeL taint-path fixpoint
         (taint-receipt (solve-ascent-camel-taint-paths
                         taint-edges taint-sources taint-sinks taint-sanitizers))
         ;; 3. ASCENT multi-hop delegation / 5-scope / breaker fixpoint
         (fix-receipt (solve-ascent-delegation-and-scope-governance
                       delegation-edges: del-edges
                       revoked-actors: rev-actors
                       context-entries: ctx-entries
                       context-flows: ctx-flows
                       target-scopes: tgt-scopes
                       declassified-entries: declass-entries
                       tripped-breakers: tripped-brks
                       breaker-tools: brk-tools))
         ;; 4. Bi-Temporal Causality cut
         (temporal-receipt
          (evaluate-temporal-causal-governance
           (list (make-temporal-governance-event "E-AUTH" 'auth 0 '() 0 0)
                 (make-temporal-governance-event "E-EXEC" 'exec 1 '("E-AUTH") 1 1))
           "E-AUTH" 1 0 5 4 3
           forbidden-event-syms: '(revoked)))
         ;; 5. Bi-Temporal Truth Maintenance reverse-dependency blast radius
         ;;    Uses caller-supplied dynamic derivation DAG when provided in slot [15],
         ;;    falling back to canonical 3-node session/tool/public derivation DAG.
         (effective-derivs
          (if (pair? custom-derivs)
            custom-derivs
            '(("D-OBO-TICKET" "pol.obo" ("human-session-alice") ())
              ("D-TOOL-EXEC" "pol.tool" ("channel-cleanliness") ("D-OBO-TICKET"))
              ("D-PUBLIC-DOC" "pol.public" ("public-workspace-policy") ()))))
         (all-subjects
          (delete-duplicates/hash
           (append (append-map caddr effective-derivs) retracted-subs)))
         (base-revs
          (map (lambda (subj)
                 (make-governance-evidence-revision
                  (string-append "rev-assert-" subj) subj 'assert #f 1
                  valid-start: 0 valid-end: 100))
               all-subjects))
         (retract-revs
          (map (lambda (subj)
                 (make-governance-evidence-revision
                  (string-append "rev-retract-" subj) subj 'retract
                  (string-append "rev-assert-" subj) 2))
               (delete-duplicates/hash retracted-subs)))
         (tm-receipt
          (evaluate-bitemporal-truth-maintenance
           "marlin-vm-evidence-journal"
           (append base-revs retract-revs)
           effective-derivs
           1 2 10))
         (retracted-conclusions (.ref tm-receipt 'affected-conclusions))
         ;; 6. Cedar Authority Snapshot synthesis (fail-closed if ANY layer rejects)
         (cedar-synth (synthesize-marlin-cedar-authority-snapshot
                       "marlin.governance.vm-bridge"
                       '("marlin.profile.governance-vm")
                       gov-receipt taint-receipt fix-receipt temporal-receipt
                       revocation-epoch: rev-epoch
                       policy-revision: pol-rev))
         (synth-policies (synthesize-cedar-policies-from-ascent fix-receipt))
         (base-reasons (.ref cedar-synth 'rejection-reasons))
         (all-reasons
          (append (map symbol->string base-reasons)
                  (if (pair? retracted-conclusions)
                    '("bitemporal-premise-retracted")
                    '())
                  (if (pair? (.ref fix-receipt 'breaker-blocked-tools))
                    '("ascent-breaker-blocked-tool")
                    '())))
         (admitted-final (and (.ref cedar-synth 'admitted?)
                              (null? retracted-conclusions)
                              (null? (.ref fix-receipt 'breaker-blocked-tools))))
         (fenced-strs (map (lambda (r) (scalar->str (car r)))
                           (.ref fix-receipt 'fenced-actors)))
         (blocked-strs (map (lambda (r) (scalar->str (car r)))
                            (.ref fix-receipt 'breaker-blocked-tools)))
         (cedar-pairs (map (lambda (p) (vector (.ref p 'identity) (.ref p 'source)))
                           synth-policies)))
    (vector
     1
     admitted-final
     rev-epoch
     pol-rev
     (.ref gov-receipt 'assessment-digest)
     (.ref taint-receipt 'proof-digest)
     (.ref fix-receipt 'proof-digest)
     (.ref temporal-receipt 'proof-digest)
     (.ref tm-receipt 'invalidation-digest)
     (list->vector all-reasons)
     (list->vector fenced-strs)
     (list->vector blocked-strs)
     (list->vector retracted-conclusions)
     (list->vector cedar-pairs))))

;; ===========================================================================
;; Contract-Validated Higher-Order Governance POO Scope
;; ===========================================================================

(def GovernanceProc (.cc Type. '.element? procedure?))

(def GovernancePolicyContract
  (.cc ObjectContract.
       responsibilities: (.o make-profile: GovernanceProc
                             compose-profiles: GovernanceProc
                             evaluate-profile: GovernanceProc
                             solve-ascent-closure: GovernanceProc
                             solve-camel-taint: GovernanceProc
                             solve-delegation-scope-fixpoint: GovernanceProc
                             run-incremental-retraction: GovernanceProc
                             evaluate-temporal-cut: GovernanceProc
                             verify-temporal-cut: GovernanceProc
                             evaluate-truth-maintenance: GovernanceProc
                             mint-effect-token: GovernanceProc
                             mediate-effect-token: GovernanceProc
                             synthesize-cedar-policies: GovernanceProc
                             synthesize-cedar-snapshot: GovernanceProc
                             make-cedar-effect-request: GovernanceProc
                             project-vector: GovernanceProc)))

(def GovernancePolicy
  (validate GovernancePolicyContract
            (.o make-profile: make-marlin-governance-profile
                compose-profiles: compose-governance-profiles
                evaluate-profile: evaluate-marlin-governance-profile
                solve-ascent-closure: solve-ascent-transitive-closure
                solve-camel-taint: solve-ascent-camel-taint-paths
                solve-delegation-scope-fixpoint: solve-ascent-delegation-and-scope-governance
                run-incremental-retraction: run-ascent-incremental-retraction-session
                evaluate-temporal-cut: evaluate-temporal-causal-governance
                verify-temporal-cut: verify-temporal-causal-governance
                evaluate-truth-maintenance: evaluate-bitemporal-truth-maintenance
                mint-effect-token: mint-governed-effect-token
                mediate-effect-token: mediate-governed-effect-token
                synthesize-cedar-policies: synthesize-cedar-policies-from-ascent
                synthesize-cedar-snapshot: synthesize-marlin-cedar-authority-snapshot
                make-cedar-effect-request: make-marlin-cedar-effect-request
                project-vector: marlin-governance-project-vector)))
