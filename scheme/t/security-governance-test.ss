;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Unit tests for Marlin's Gerbil Scheme POO Security Governance policy plane:
;;;   - 5-Scope Shared Context Governance (Private / DirectMessage / Channel / Workspace / External)
;;;   - Multi-tenant isolation, owner-only scope promotion & declassification, monotonic ACL epochs
;;;   - Anti-exfiltration prompt scope flow
;;;   - AI/Tool Execution Circuit Breaker ("脱扣" / E-Stop), lease fencing, budget cap,
;;;     human-only re-arm, and CaMeL IFC capability checks.

(import :std/test
        (only-in :clan/poo/object .get .o)
        "../policy/security")

(export marlin-security-governance-test)

(def (mk-human id tenant)
  (.o principal-id: id
      effective-owner-id: id
      tenant-id: tenant
      is-agent?: #f))

(def (mk-agent agent-name acting-for-id tenant)
  (.o principal-id: agent-name
      effective-owner-id: acting-for-id
      tenant-id: tenant
      is-agent?: #t))

(def (mk-entry entry-scope entry-sensitivity entry-declassified?
               owner-id: (entry-owner "alice")
               tenant-id: (entry-tenant "tenant-a")
               channel-id: (entry-channel "ch-sec")
               dm-participants: (entry-dm-participants '("bob")))
  (.o owner-id: entry-owner
      tenant-id: entry-tenant
      channel-id: entry-channel
      dm-participants: entry-dm-participants
      scope: entry-scope
      sensitivity: entry-sensitivity
      declassified?: entry-declassified?))

(def (clean-ifc)
  (.o plan-integrity-high?: #t
      tool-controlled-by-tainted-data?: #f
      carries-confidential-data?: #f
      declassification-approved?: #f))

(def marlin-security-governance-test
  (test-suite "Marlin Backend Scheme POO Security Governance"

    (test-case "5-scope visibility lattice ranks strictly from private to external"
      (let ((rank (.get SecurityGovernancePolicy scope-rank)))
        (check (rank 'private) => 0)
        (check (rank 'direct-message) => 1)
        (check (rank 'channel) => 2)
        (check (rank 'workspace) => 3)
        (check (rank 'external) => 4)
        (check (rank 'unknown) => #f)))

    (test-case "private, DM, cross-tenant, and channel ACL-epoch revocation fail closed"
      (let* ((alice (mk-human "alice" "tenant-a"))
             (bob (mk-human "bob" "tenant-a"))
             (carol (mk-human "carol" "tenant-a"))
             (mallory-b (mk-human "alice" "tenant-b"))
             (priv (mk-entry 'private 'confidential #f))
             (dm (mk-entry 'direct-message 'internal #f dm-participants: '("bob")))
             (chan-entry (mk-entry 'channel 'internal #t))
             (ch-epoch-1 (.o acl-epoch: 1 members: '("alice" "bob") tainted?: #f))
             (ch-epoch-2 (.o acl-epoch: 2 members: '("alice") tainted?: #f))
             (bob-handle-1 (.o tenant-id: "tenant-a" principal-id: "bob" channel-id: "ch-sec" acl-epoch: 1)))
        ;; Private: owner allowed, peer & cross-tenant denied
        (check (.get (evaluate-shared-context-read alice priv) allowed?) => #t)
        (check (.get (evaluate-shared-context-read bob priv) reason) => 'private-scope-denied)
        (check (.get (evaluate-shared-context-read mallory-b priv) reason) => 'cross-tenant-denied)
        ;; DM: participant bob allowed, outsider carol denied
        (check (.get (evaluate-shared-context-read bob dm) allowed?) => #t)
        (check (.get (evaluate-shared-context-read carol dm) reason) => 'direct-message-participant-denied)
        ;; Channel: active member with epoch 1 handle allowed; after revocation to epoch 2 denied
        (check (.get (evaluate-shared-context-read bob chan-entry channel-state: ch-epoch-1 handle: bob-handle-1) allowed?) => #t)
        (check (.get (evaluate-shared-context-read bob chan-entry channel-state: ch-epoch-2 handle: bob-handle-1) reason) => 'not-channel-member)
        ;; Even if bob rejoins at epoch 2, pre-revocation epoch-1 handle stays permanently stale
        (let ((ch-rejoined (.o acl-epoch: 2 members: '("alice" "bob") tainted?: #f)))
          (check (.get (evaluate-shared-context-read bob chan-entry channel-state: ch-rejoined handle: bob-handle-1) reason)
                 => 'stale-handle-epoch))))

    (test-case "owner-only scope promotion, agent refusal, confidential declassification, and anti-exfiltration"
      (let* ((alice (mk-human "alice" "tenant-a"))
             (bob (mk-human "bob" "tenant-a"))
             (agent-alice (mk-agent "agent-1" "alice" "tenant-a"))
             (priv-secret (mk-entry 'private 'confidential #f)))
        ;; Agent cannot promote scope even when acting for owner
        (check (.get (evaluate-shared-context-promotion agent-alice priv-secret 'channel #t) reason)
               => 'agent-privilege-denied)
        ;; Non-owner human cannot promote
        (check (.get (evaluate-shared-context-promotion bob priv-secret 'channel #t) reason)
               => 'non-owner-promotion-denied)
        ;; Owner without explicit declassification cannot promote confidential entry
        (check (.get (evaluate-shared-context-promotion alice priv-secret 'channel #f) reason)
               => 'declassification-required)
        ;; Owner with explicit declassification succeeds
        (check (.get (evaluate-shared-context-promotion alice priv-secret 'channel #t) allowed?)
               => #t)
        ;; Anti-exfiltration: Private or DM cannot flow into Channel/External prompt
        (check (.get (evaluate-prompt-scope-flow 'private 'channel) reason)
               => 'prompt-exfiltration-blocked)
        (check (.get (evaluate-prompt-scope-flow 'direct-message 'workspace) reason)
               => 'prompt-exfiltration-blocked)
        (check (.get (evaluate-prompt-scope-flow 'workspace 'channel) allowed?)
               => #t)))

    (test-case "multi-party tainted shared channel requires verified human gate for tool call"
      (let ((clean-ch (.o tainted?: #f))
            (tainted-ch (.o tainted?: #t)))
        (check (.get (evaluate-shared-context-tool-gate clean-ch #f) allowed?) => #t)
        (check (.get (evaluate-shared-context-tool-gate tainted-ch #f) reason)
               => 'tainted-context-requires-human-gate)
        (check (.get (evaluate-shared-context-tool-gate tainted-ch #t) allowed?) => #t)))

    (test-case "execution circuit breaker blocks tripped, stale-epoch, budget-overrun, and CaMeL IFC violations"
      (let* ((armed (.o tripped?: #f trip-epoch: 2 budget-used: 0 budget-limit: 1))
             (tripped (.o tripped?: #t trip-epoch: 2 budget-used: 0 budget-limit: 1))
             (budget-full (.o tripped?: #f trip-epoch: 2 budget-used: 1 budget-limit: 1))
             (fresh-lease (.o active?: #t trip-epoch: 2))
             (stale-lease (.o active?: #t trip-epoch: 1))
             (tainted-ifc (.o plan-integrity-high?: #t
                              tool-controlled-by-tainted-data?: #t
                              carries-confidential-data?: #f
                              declassification-approved?: #f))
             (secret-no-declass (.o plan-integrity-high?: #t
                                    tool-controlled-by-tainted-data?: #f
                                    carries-confidential-data?: #t
                                    declassification-approved?: #f))
             (secret-declass (.o plan-integrity-high?: #t
                                 tool-controlled-by-tainted-data?: #f
                                 carries-confidential-data?: #t
                                 declassification-approved?: #t)))
        (check (.get (evaluate-execution-breaker-commit armed fresh-lease (clean-ifc)) allowed?) => #t)
        (check (.get (evaluate-execution-breaker-commit tripped fresh-lease (clean-ifc)) reason) => 'breaker-tripped)
        (check (.get (evaluate-execution-breaker-commit armed stale-lease (clean-ifc)) reason) => 'stale-lease-epoch)
        (check (.get (evaluate-execution-breaker-commit budget-full fresh-lease (clean-ifc)) reason) => 'budget-exhausted)
        (check (.get (evaluate-execution-breaker-commit armed fresh-lease tainted-ifc) reason)
               => 'prompt-injection-control-hijack-denied)
        (check (.get (evaluate-execution-breaker-commit armed fresh-lease secret-no-declass) reason)
               => 'confidential-exfiltration-denied)
        (check (.get (evaluate-execution-breaker-commit armed fresh-lease secret-declass) allowed?) => #t)))

    (test-case "execution circuit breaker re-arm is strictly verified-human-only and bumps trip-epoch"
      (let* ((tripped (.o tripped?: #t trip-epoch: 2))
             (armed (.o tripped?: #f trip-epoch: 2))
             (agent (.o actor-id: "agent-0" is-agent?: #t verified-human?: #f))
             (unverified (.o actor-id: "bob" is-agent?: #f verified-human?: #f))
             (human (.o actor-id: "ops-human" is-agent?: #f verified-human?: #t)))
        (check (.get (evaluate-execution-breaker-rearm armed human) reason) => 'breaker-not-tripped)
        (check (.get (evaluate-execution-breaker-rearm tripped agent) reason) => 'human-only-rearm-required)
        (check (.get (evaluate-execution-breaker-rearm tripped unverified) reason) => 'human-only-rearm-required)
        (let ((ok (evaluate-execution-breaker-rearm tripped human)))
          (check (.get ok allowed?) => #t)
          (check (.get ok detail) => 3))))))
