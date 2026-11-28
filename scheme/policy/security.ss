;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Pure functional POO security governance policies for:
;;;   1. 5-Scope Shared Context Governance (Private / DirectMessage / Channel / Workspace / External),
;;;      multi-tenant isolation, owner-only scope promotion & declassification, monotonic ACL epochs,
;;;      and multi-party CaMeL taint gating.
;;;   2. Authoritative AI/Tool Execution Circuit Breaker ("脱扣" / E-Stop),
;;;      trip-epoch lease fencing, armed-window budget caps, human-only re-arm,
;;;      and CaMeL control-data segregation / confidential egress gating.
;;;
;;; Zero side effects, zero text serialization: pure POO values and sealed decision receipts.

(import (only-in :clan/poo/object .cc .o .get)
        (only-in :clan/poo/type Type. validate)
        (only-in :clan/poo/types/object-contract ObjectContract.))

(export SecurityGovernanceContract
        SecurityGovernancePolicy
        shared-context-scope-rank
        evaluate-shared-context-read
        evaluate-shared-context-promotion
        evaluate-shared-context-tool-gate
        evaluate-prompt-scope-flow
        evaluate-execution-breaker-commit
        evaluate-execution-breaker-rearm
        evaluate-delegated-obo-authz
        evaluate-multi-tenant-fence
        evaluate-chatroom-ingress
        evaluate-governance-sod)

;; ---------------------------------------------------------------------------
;; 1. 5-Scope Shared Context Governance (pure lattice & decision evaluators)
;; ---------------------------------------------------------------------------

(def (shared-context-scope-rank scope)
  (case scope
    ((private) 0)
    ((direct-message) 1)
    ((channel) 2)
    ((workspace) 3)
    ((external) 4)
    (else #f)))

(def (make-security-decision allowed-flag reason-sym detail: (detail-val #f))
  (.o kind: 'marlin.security.policy-decision.v1
      allowed?: (and allowed-flag #t)
      reason: reason-sym
      detail: detail-val))

;; Evaluates whether `actor` may read `entry` under `channel-state` and optional `handle`.
(def (evaluate-shared-context-read actor entry
                                   channel-state: (channel-state #f)
                                   handle: (handle #f))
  (let* ((actor-tenant (.get actor tenant-id))
         (entry-tenant (.get entry tenant-id))
         (effective-id (.get actor effective-owner-id))
         (owner-id (.get entry owner-id))
         (scope (.get entry scope))
         (rank (shared-context-scope-rank scope)))
    (cond
      ((not rank)
       (make-security-decision #f 'invalid-scope detail: scope))
      ((not (equal? actor-tenant entry-tenant))
       (make-security-decision #f 'cross-tenant-denied detail: actor-tenant))
      ((eq? scope 'private)
       (if (equal? effective-id owner-id)
         (make-security-decision #t 'allowed-private-owner)
         (make-security-decision #f 'private-scope-denied detail: effective-id)))
      ((eq? scope 'direct-message)
       (if (or (equal? effective-id owner-id)
               (and (member effective-id (.get entry dm-participants)) #t))
         (make-security-decision #t 'allowed-direct-message-participant)
         (make-security-decision #f 'direct-message-participant-denied detail: effective-id)))
      ((eq? scope 'channel)
       (cond
         ((not channel-state)
          (make-security-decision #f 'channel-state-required))
         ((not handle)
          (make-security-decision #f 'channel-handle-required))
         ((not (and (equal? (.get handle tenant-id) actor-tenant)
                    (equal? (.get handle principal-id) effective-id)
                    (equal? (.get handle channel-id) (.get entry channel-id))))
          (make-security-decision #f 'handle-binding-mismatch))
         ((not (and (member effective-id (.get channel-state members)) #t))
          (make-security-decision #f 'not-channel-member detail: effective-id))
         ((not (= (.get handle acl-epoch) (.get channel-state acl-epoch)))
          (make-security-decision #f 'stale-handle-epoch
                                  detail: (cons (.get handle acl-epoch)
                                                (.get channel-state acl-epoch))))
         (else
          (make-security-decision #t 'allowed-channel-member
                                  detail: (.get channel-state acl-epoch)))))
      ((eq? scope 'workspace)
       (make-security-decision #t 'allowed-workspace-member))
      ((eq? scope 'external)
       (if (and (eq? (.get entry sensitivity) 'confidential)
                (not (.get entry declassified?)))
         (make-security-decision #f 'declassification-required)
         (make-security-decision #t 'allowed-external)))
      (else
       (make-security-decision #f 'deny-default)))))

;; Evaluates whether `actor` may promote `entry` to `target-scope`.
;; Agents are unconditionally denied; only the human owner may promote, and
;; confidential entries require explicit declassification.
(def (evaluate-shared-context-promotion actor entry target-scope explicit-declassify?)
  (let* ((from-rank (shared-context-scope-rank (.get entry scope)))
         (to-rank (shared-context-scope-rank target-scope)))
    (cond
      ((.get actor is-agent?)
       (make-security-decision #f 'agent-privilege-denied detail: 'promote-context-scope))
      ((not (equal? (.get actor tenant-id) (.get entry tenant-id)))
       (make-security-decision #f 'cross-tenant-denied))
      ((not (equal? (.get actor principal-id) (.get entry owner-id)))
       (make-security-decision #f 'non-owner-promotion-denied))
      ((or (not from-rank) (not to-rank) (<= to-rank from-rank))
       (make-security-decision #f 'invalid-scope-transition
                               detail: (cons (.get entry scope) target-scope)))
      ((and (eq? (.get entry sensitivity) 'confidential)
            (not (.get entry declassified?))
            (not explicit-declassify?))
       (make-security-decision #f 'declassification-required))
      (else
       (make-security-decision #t 'allowed-scope-promotion detail: target-scope)))))

;; Evaluates anti-exfiltration flow: narrower context (e.g. Private/DM) must never
;; flow into a broader target prompt scope (e.g. Channel/Workspace/External).
(def (evaluate-prompt-scope-flow entry-scope target-scope)
  (let ((entry-rank (shared-context-scope-rank entry-scope))
        (target-rank (shared-context-scope-rank target-scope)))
    (cond
      ((or (not entry-rank) (not target-rank))
       (make-security-decision #f 'invalid-scope))
      ((< entry-rank target-rank)
       (make-security-decision #f 'prompt-exfiltration-blocked
                               detail: (cons entry-scope target-scope)))
      (else
       (make-security-decision #t 'allowed-scope-flow)))))

;; Evaluates CaMeL multi-party taint gate before tool execution in a shared channel.
(def (evaluate-shared-context-tool-gate channel-state human-gate-approved?)
  (if (and (.get channel-state tainted?) (not human-gate-approved?))
    (make-security-decision #f 'tainted-context-requires-human-gate)
    (make-security-decision #t 'allowed-channel-tool-call)))

;; ---------------------------------------------------------------------------
;; 2. Authoritative AI/Tool Execution Circuit Breaker ("脱扣" / E-Stop)
;; ---------------------------------------------------------------------------

;; Evaluates Phase-2 side-effect commit against breaker latch, monotonic trip-epoch,
;; armed-window tool budget cap, and CaMeL IFC capability checks.
(def (evaluate-execution-breaker-commit breaker-state lease ifc-context)
  (cond
    ((.get breaker-state tripped?)
     (make-security-decision #f 'breaker-tripped
                             detail: (.get breaker-state trip-epoch)))
    ((not (.get lease active?))
     (make-security-decision #f 'lease-not-active))
    ((not (= (.get lease trip-epoch) (.get breaker-state trip-epoch)))
     (make-security-decision #f 'stale-lease-epoch
                             detail: (cons (.get lease trip-epoch)
                                           (.get breaker-state trip-epoch))))
    ((>= (.get breaker-state budget-used) (.get breaker-state budget-limit))
     (make-security-decision #f 'budget-exhausted
                             detail: (cons (.get breaker-state budget-used)
                                           (.get breaker-state budget-limit))))
    ((not (.get ifc-context plan-integrity-high?))
     (make-security-decision #f 'low-integrity-plan-denied))
    ((.get ifc-context tool-controlled-by-tainted-data?)
     (make-security-decision #f 'prompt-injection-control-hijack-denied))
    ((and (.get ifc-context carries-confidential-data?)
          (not (.get ifc-context declassification-approved?)))
     (make-security-decision #f 'confidential-exfiltration-denied))
    (else
     (make-security-decision #t 'allowed-tool-commit
                             detail: (+ (.get breaker-state budget-used) 1)))))

;; Evaluates human-only re-arm control after emergency stop.
(def (evaluate-execution-breaker-rearm breaker-state actor)
  (cond
    ((not (.get breaker-state tripped?))
     (make-security-decision #f 'breaker-not-tripped))
    ((or (.get actor is-agent?) (not (.get actor verified-human?)))
     (make-security-decision #f 'human-only-rearm-required
                             detail: (.get actor actor-id)))
    (else
     (make-security-decision #t 'allowed-human-rearm
                             detail: (+ (.get breaker-state trip-epoch) 1)))))

;; ---------------------------------------------------------------------------
;; 3. OBO Delegated Agent Authz, Multi-Tenant Fence, Chatroom Ingress, & SoD
;; ---------------------------------------------------------------------------

;; RFC 8693 OBO delegation + RFC 9470 step-up verification floor evaluator.
(def (evaluate-delegated-obo-authz session request)
  (let ((action (.get request action)))
    (cond
      ((not (.get session signed-in?))
       (make-security-decision #f 'human-session-signed-out))
      ((eq? action 'delegate)
       (if (> (.get request requested-scope) (.get session human-scope-level))
         (make-security-decision #f 'scope-amplification-denied
                                 detail: (cons (.get request requested-scope)
                                               (.get session human-scope-level)))
         (make-security-decision #t 'allowed-obo-delegation
                                 detail: (.get session auth-epoch))))
      ((eq? action 'execute-delegated)
       (if (not (= (.get request ticket-epoch) (.get session auth-epoch)))
         (make-security-decision #f 'zombie-delegation-denied
                                 detail: (cons (.get request ticket-epoch)
                                               (.get session auth-epoch)))
         (make-security-decision #t 'allowed-delegated-execution)))
      ((eq? action 'approve-human-gate)
       (if (.get request is-agent?)
         (make-security-decision #f 'agent-impersonation-denied detail: 'approve-human-gate)
         (make-security-decision #t 'allowed-human-gate-approval)))
      ((eq? action 'execute-privileged-human)
       (cond
         ((.get request is-agent?)
          (make-security-decision #f 'agent-impersonation-denied detail: 'execute-privileged-human))
         ((or (not (.get session email-verified?))
              (not (.get session step-up-fresh?)))
          (make-security-decision #f 'step-up-verification-required))
         (else
          (make-security-decision #t 'allowed-privileged-step-up))))
      (else
       (make-security-decision #f 'unknown-obo-action detail: action)))))

;; Multi-tenant isolation & tenant-pinned offline outbox fence evaluator.
(def (evaluate-multi-tenant-fence tenant-state request)
  (let ((action (.get request action)))
    (cond
      ((eq? action 'enforce-ticket)
       (cond
         ((not (equal? (.get request ticket-tenant) (.get request target-tenant)))
          (make-security-decision #f 'cross-tenant-deputy-denied))
         ((.get tenant-state revoked?)
          (make-security-decision #f 'revoked-tenant-ticket-denied))
         ((not (= (.get request ticket-epoch) (.get tenant-state epoch)))
          (make-security-decision #f 'revoked-tenant-ticket-denied
                                  detail: (cons (.get request ticket-epoch)
                                                (.get tenant-state epoch))))
         (else
          (make-security-decision #t 'allowed-tenant-execution))))
      ((eq? action 'flush-outbox)
       (cond
         ((not (equal? (.get request outbox-tenant) (.get request active-tenant)))
          (make-security-decision #f 'cross-tenant-outbox-flush-denied))
         ((.get tenant-state revoked?)
          (make-security-decision #f 'tenant-revoked))
         (else
          (make-security-decision #t 'allowed-outbox-flush))))
      (else
       (make-security-decision #f 'unknown-tenant-action detail: action)))))

;; Multi-party chatroom ingress, channel affinity, one-shot gate, and taint gate evaluator.
(def (evaluate-chatroom-ingress room-state request)
  (let ((action (.get request action)))
    (cond
      ((eq? action 'ingest-external)
       (cond
         ((or (not (.get request origin-verified?))
              (.get request claims-gate-receipt?))
          (make-security-decision #f 'forged-external-ingress-denied))
         ((not (equal? (.get request target-channel) (.get request thread-root-channel)))
          (make-security-decision #f 'cross-channel-injection-denied))
         (else
          (make-security-decision #t 'allowed-tainted-ingress))))
      ((eq? action 'resolve-gate)
       (cond
         ((.get request is-agent?)
          (make-security-decision #f 'agent-impersonation-denied detail: 'resolve-gate))
         ((not (eq? (.get room-state gate-status) 'pending))
          (make-security-decision #f 'gate-replay-denied detail: (.get room-state gate-status)))
         (else
          (make-security-decision #t 'allowed-human-gate-resolution))))
      ((eq? action 'unblock-matter)
       (if (.get request is-agent?)
         (make-security-decision #f 'agent-self-unblock-denied)
         (make-security-decision #t 'allowed-human-unblock)))
      ((eq? action 'invoke-tool)
       (if (and (.get room-state tainted?)
                (not (eq? (.get room-state gate-status) 'approved)))
         (make-security-decision #f 'tainted-chatroom-tool-bypass-denied)
         (make-security-decision #t 'allowed-chatroom-tool)))
      (else
       (make-security-decision #f 'unknown-chatroom-action detail: action)))))

;; Dual-control Separation of Duties (SoD) & veto precedence evaluator.
(def (evaluate-governance-sod proposal request)
  (let ((action (.get request action)))
    (cond
      ((eq? action 'approve)
       (cond
         ((.get proposal vetoed?)
          (make-security-decision #f 'vetoed-proposal-denied))
         ((equal? (.get proposal proposer) (.get request actor-id))
          (make-security-decision #f 'sod-self-approval-denied))
         (else
          (make-security-decision #t 'allowed-sod-approval))))
      ((eq? action 'execute)
       (cond
         ((.get proposal vetoed?)
          (make-security-decision #f 'vetoed-proposal-denied))
         ((not (.get proposal approver))
          (make-security-decision #f 'proposal-not-approved))
         ((equal? (.get proposal proposer) (.get proposal approver))
          (make-security-decision #f 'sod-self-approval-denied))
         (else
          (make-security-decision #t 'allowed-sod-execution))))
      (else
       (make-security-decision #f 'unknown-sod-action detail: action)))))

;; ---------------------------------------------------------------------------
;; 4. Contract-validated POO Security Governance Bundle
;; ---------------------------------------------------------------------------

(def SecurityPolicyOperation (.cc Type. '.element? procedure?))

(def SecurityGovernanceContract
  (.cc ObjectContract.
       responsibilities: (.o scope-rank: SecurityPolicyOperation
                             evaluate-context-read: SecurityPolicyOperation
                             evaluate-context-promotion: SecurityPolicyOperation
                             evaluate-prompt-flow: SecurityPolicyOperation
                             evaluate-channel-tool-gate: SecurityPolicyOperation
                             evaluate-breaker-commit: SecurityPolicyOperation
                             evaluate-breaker-rearm: SecurityPolicyOperation
                             evaluate-obo-authz: SecurityPolicyOperation
                             evaluate-multi-tenant: SecurityPolicyOperation
                             evaluate-chatroom: SecurityPolicyOperation
                             evaluate-sod: SecurityPolicyOperation)))

(def SecurityGovernancePolicy
  (validate SecurityGovernanceContract
            (.o scope-rank: shared-context-scope-rank
                evaluate-context-read: evaluate-shared-context-read
                evaluate-context-promotion: evaluate-shared-context-promotion
                evaluate-prompt-flow: evaluate-prompt-scope-flow
                evaluate-channel-tool-gate: evaluate-shared-context-tool-gate
                evaluate-breaker-commit: evaluate-execution-breaker-commit
                evaluate-breaker-rearm: evaluate-execution-breaker-rearm
                evaluate-obo-authz: evaluate-delegated-obo-authz
                evaluate-multi-tenant: evaluate-multi-tenant-fence
                evaluate-chatroom: evaluate-chatroom-ingress
                evaluate-sod: evaluate-governance-sod)))
