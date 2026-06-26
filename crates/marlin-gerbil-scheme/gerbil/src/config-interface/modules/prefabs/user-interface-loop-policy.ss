;;; -*- Gerbil -*-
;;; Boundary: Marlin-owned loop governor policy for the user-interface prefab.

package: config-interface/modules/prefabs

(import (only-in :clan/poo/object .get .o)
        :poo-flow/src/loops/agent
        :marlin/deck-runtime
        :config-interface/modules/lib
        :config-interface/modules/loop-engine-policy
        :config-interface/modules/prefabs/user-interface-config)

(export user-interface-marlin-loops-policy-kind
        UserInterfaceMarlinLoopsPolicy
        UserInterfaceLoopGovernorPattern
        UserInterfaceLoopGovernorStrategy
        UserInterfaceLoopGovernor
        UserInterfaceLoopGovernorStateFacts
        UserInterfaceLoopGovernorRequestEnvelope
        UserInterfaceLoopGovernorL1Receipt
        UserInterfaceLoopGovernorRuntimeManifest)

;;; Boundary: This is Marlin's own loop policy, not the upstream example file.
;; : String
(def user-interface-marlin-loops-policy-kind
  "marlin.config-interface.prefabs.user-interface.loops-policy.v1")

;;; Boundary: Marlin owns policy intent; poo-flow projects the control plane.
;; : (-> POOObject POOObject)
(def (UserInterfaceMarlinLoopsPolicy config)
  (let ((workspace-root-value (user-interface-workspace-root config))
        (receipt-contracts-value
         (marlinDefaultLoopEngineReceiptContracts)))
    (.o kind: user-interface-marlin-loops-policy-kind
        id: "user-interface-marlin-loops-policy"
        owner: "marlin"
        source: "config-interface/modules/prefabs/user-interface#loops-policy"
        reference-role: "marlin-owned-loops-policy"
        upstream-example-role: "poo-flow-user-interface-reference-only"
        control-plane-owner: "poo-flow"
        runtime-execution-owner: "marlin-agent-core"
        runtime-effect: "handoff-only"
        loop-name: 'user-interface-policy-loop
        governor-id: 'user-interface-policy-governor
        strategy-id: 'user-interface-policy-strategy
        summary:
        "Report user-interface policy handoff readiness for Marlin runtime."
        level: 'l2
        priority: 1
        workspace-root: workspace-root-value
        budget: '((max-attempts . 1) (max-actionable . 1))
        isolation: '((mode . workspace))
        maker: '((enabled . #f))
        checker: '((required . #t))
        capabilities: '(+manifest-handoff +l1-receipts)
        open-patterns: '(user-interface-policy-loop)
        blocked-patterns: '()
        receipt-contracts: receipt-contracts-value
        receipt-family-ids:
        (map (lambda (contract)
               (user-interface-contract-field contract 'id))
             receipt-contracts-value)
        receipt-schema-ids:
        (map (lambda (contract)
               (user-interface-contract-field contract 'schema))
             receipt-contracts-value))))

;;; Boundary: Loop governor intent is projected by poo-flow from Marlin policy.
;;; Marlin consumes the projected runtime manifest and still owns execution.
;; : (-> POOObject POOObject)
(def (UserInterfaceLoopGovernorPattern config)
  (let (loops-policy (UserInterfaceMarlinLoopsPolicy config))
    (make-loop-pattern-descriptor
     (.get loops-policy loop-name)
     (.get loops-policy summary)
     (list (cons 'level (.get loops-policy level))
           (cons 'priority (.get loops-policy priority))
           (cons 'watched-scope
                 (list (.get loops-policy workspace-root)))
           (cons 'budget (.get loops-policy budget))
           (cons 'isolation (.get loops-policy isolation))
           (cons 'maker (.get loops-policy maker))
           (cons 'checker (.get loops-policy checker))
           (cons 'metadata
                 (list (cons 'acting_on
                             (.get loops-policy workspace-root))
                       (cons 'source
                             (.get loops-policy source))
                       (cons 'module-system
                             (.get loops-policy control-plane-owner))
                       (cons 'policy-owner
                             (.get loops-policy owner))))))))

;;; Boundary: Strategy composition stays in poo-flow; this is inert policy data.
;; : (-> POOObject POOObject)
(def (UserInterfaceLoopGovernorStrategy config)
  (let (loops-policy (UserInterfaceMarlinLoopsPolicy config))
    (make-loop-strategy-plan
     (.get loops-policy strategy-id)
     (list (UserInterfaceLoopGovernorPattern config))
     (list (cons 'level-ceiling (.get loops-policy level))
           (cons 'metadata
                 (list (cons 'source (.get loops-policy source))
                       (cons 'control-plane
                             (.get loops-policy control-plane-owner))
                       (cons 'policy-owner
                             (.get loops-policy owner))))))))

;;; Boundary: Governor is the Marlin handoff contract, not a scheduler.
;; : (-> POOObject POOObject)
(def (UserInterfaceLoopGovernor config)
  (let (loops-policy (UserInterfaceMarlinLoopsPolicy config))
    (make-loop-governor
     (.get loops-policy governor-id)
     (UserInterfaceLoopGovernorStrategy config)
     (list
      (cons 'metadata
            (list (cons 'source (.get loops-policy source))
                  (cons 'control-plane
                        (.get loops-policy control-plane-owner))
                  (cons 'execution-owner
                        (.get loops-policy runtime-execution-owner))
                  (cons 'policy-owner
                        (.get loops-policy owner))))))))

;;; Boundary: Runtime state facts are supplied as data for projection only.
;; : (-> POOObject List)
(def (UserInterfaceLoopGovernorStateFacts _config)
  '())

;;; Boundary: Request envelope is produced by poo-flow and consumed by Marlin.
;; : (-> POOObject POOObject)
(def (UserInterfaceLoopGovernorRequestEnvelope config)
  (let (loops-policy (UserInterfaceMarlinLoopsPolicy config))
    (loop-governor->marlin-request-envelope
     (UserInterfaceLoopGovernor config)
     (UserInterfaceLoopGovernorStateFacts config)
     (.get loops-policy governor-id))))

;;; Boundary: L1 receipt proves report-only handoff without local effects.
;; : (-> POOObject POOObject)
(def (UserInterfaceLoopGovernorL1Receipt config)
  (let (loops-policy (UserInterfaceMarlinLoopsPolicy config))
    (loop-governor->l1-run-receipt
     (UserInterfaceLoopGovernor config)
     (UserInterfaceLoopGovernorStateFacts config)
     (.get loops-policy governor-id))))

;;; Boundary: Runtime manifest is the stable discovery surface for Rust/debug.
;; : (-> POOObject POOObject)
(def (UserInterfaceLoopGovernorRuntimeManifest config)
  (let (loops-policy (UserInterfaceMarlinLoopsPolicy config))
    (loop-governor->marlin-runtime-manifest
     (UserInterfaceLoopGovernor config)
     (UserInterfaceLoopGovernorStateFacts config)
     (.get loops-policy governor-id))))
