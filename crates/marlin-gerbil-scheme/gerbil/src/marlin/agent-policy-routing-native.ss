;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy/runtime wrapper logic over the native ABI.

(import (only-in :std/foreign
                 begin-foreign
                 c-define
                 extern
                 int))

(include "./_agent-policy-routing-native.ssi")

(declare
  (block)
  (standard-bindings)
  (extended-bindings)
  (not safe))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define marlin-agent-policy-routing-native-abi-id
  "marlin.agent.policy-routing.native")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define marlin-agent-policy-routing-native-abi-version 1)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define marlin-agent-policy-routing-native-status-abi-mismatch 3)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define marlin-agent-policy-routing-native-symbol
  "marlin_agent_policy_routing_select_edges")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-string-list len ref)
  (map ref (list-tabulate len identity)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-request-candidate-edges request)
  (native-string-list
   (native-request-candidate-edges-len request)
   (lambda (index) (native-request-candidate-edge-at request index))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-routing-evidence kind evidence-id)
  [kind evidence-id])

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-request-routing-evidence request)
  (map (lambda (index)
         (native-routing-evidence
          (native-request-routing-evidence-kind-at request index)
          (native-request-routing-evidence-id-at request index)))
       (list-tabulate (native-request-routing-evidence-len request) identity)))

(extern marlin-agent-policy-routing-native-select-edges)
(begin-foreign
  (namespace ("marlin-deck-runtime/src/marlin/agent-policy-routing-native#"
              marlin-agent-policy-routing-native-abi-version
              marlin-agent-policy-routing-native-status-abi-mismatch
              native-request-abi-version
              native-request-graph-id
              native-request-policy-scope
              native-request-root-node
              native-request-candidate-edges
              native-request-routing-evidence
              marlin-agent-policy-routing-select-edges
              marlin-agent-policy-routing-projection-decision
              native-set-projection!))

  (c-define (marlin-agent-policy-routing-native-select-edges request projection)
    (agent-policy-routing-request* agent-policy-routing-projection*) int
    "marlin_agent_policy_routing_select_edges" ""
    (if (not (= (native-request-abi-version request)
                marlin-agent-policy-routing-native-abi-version))
      marlin-agent-policy-routing-native-status-abi-mismatch
      (native-set-projection! projection "select_edges"))))
