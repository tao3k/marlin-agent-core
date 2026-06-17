;;; -*- Gerbil -*-
;;; Boundary: Module owns the Gerbil native C ABI for AgentGraph policy routing.

(import (only-in :std/foreign
                 begin-ffi
                 begin-foreign
                 c-define
                 c-define-type
                 c-declare
                 char-string
                 define-c-lambda
                 extern
                 int
                 pointer)
        (only-in :marlin/agent-policy-routing-native-projection
                 make-marlin-agent-policy-routing-evidence
                 marlin-agent-policy-routing-projection-decision
                 marlin-agent-policy-routing-select-edges))

(export marlin-agent-policy-routing-native-abi-id
        marlin-agent-policy-routing-native-abi-version
        marlin-agent-policy-routing-native-status-abi-mismatch
        marlin-agent-policy-routing-native-symbol)

(declare
  (block)
  (standard-bindings)
  (extended-bindings)
  (not safe))

(begin-ffi (native-request-abi-version
            native-request-graph-id
            native-request-policy-scope
            native-request-root-node
            native-request-candidate-edges-len
            native-request-candidate-edge-at
            native-request-routing-evidence-len
            native-request-routing-evidence-kind-at
            native-request-routing-evidence-id-at
            native-set-projection!)
  (c-declare #<<END-C
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION 1u
#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK 0
#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER 2
#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH 3
#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION 4

typedef struct MarlinAgentPolicyRoutingUtf8 {
  const uint8_t *ptr;
  size_t len;
} MarlinAgentPolicyRoutingUtf8;

typedef struct MarlinAgentPolicyRoutingUtf8List {
  const MarlinAgentPolicyRoutingUtf8 *items;
  size_t len;
} MarlinAgentPolicyRoutingUtf8List;

typedef struct MarlinAgentPolicyRoutingEvidence {
  MarlinAgentPolicyRoutingUtf8 evidence_kind;
  MarlinAgentPolicyRoutingUtf8 evidence_id;
} MarlinAgentPolicyRoutingEvidence;

typedef struct MarlinAgentPolicyRoutingEvidenceList {
  const MarlinAgentPolicyRoutingEvidence *items;
  size_t len;
} MarlinAgentPolicyRoutingEvidenceList;

typedef struct MarlinAgentPolicyRoutingRequest {
  uint32_t abi_version;
  MarlinAgentPolicyRoutingUtf8 graph_id;
  MarlinAgentPolicyRoutingUtf8 policy_scope;
  MarlinAgentPolicyRoutingUtf8 root_node;
  MarlinAgentPolicyRoutingUtf8List candidate_edges;
  MarlinAgentPolicyRoutingEvidenceList routing_evidence;
} MarlinAgentPolicyRoutingRequest;

typedef struct MarlinAgentPolicyRoutingProjection {
  uint32_t abi_version;
  MarlinAgentPolicyRoutingUtf8 routing_decision;
} MarlinAgentPolicyRoutingProjection;

static char *marlin_agent_policy_routing_native_scratch = NULL;
static size_t marlin_agent_policy_routing_native_scratch_capacity = 0;
static char *marlin_agent_policy_routing_native_decision = NULL;
static size_t marlin_agent_policy_routing_native_decision_capacity = 0;
static int marlin_agent_policy_routing_native_initialized = 0;

___BEGIN_NEW_LNK
___DEF_NEW_LNK(___LNK_agent_2d_policy_2d_routing_2d_native_7e_0__)
___END_NEW_LNK

int
marlin_agent_policy_routing_initialize (void)
{
  ___SCMOBJ setup_result;
  ___setup_params_struct setup_params;

  if (marlin_agent_policy_routing_native_initialized != 0) {
    return 0;
  }

  ___setup_params_reset (&setup_params);
  setup_params.version = ___VERSION;
  setup_params.linker = ___LNK_agent_2d_policy_2d_routing_2d_native_7e_0__;
  setup_result = ___setup (&setup_params);
  if (setup_result != ___FIX(___NO_ERR)) {
    return ___INT(setup_result);
  }

  marlin_agent_policy_routing_native_initialized = 1;
  return 0;
}

static char*
marlin_agent_policy_routing_utf8_to_c_string (MarlinAgentPolicyRoutingUtf8 value)
{
  size_t len = value.len;
  if (value.ptr == NULL && len != 0) {
    return "";
  }
  if (len + 1 > marlin_agent_policy_routing_native_scratch_capacity) {
    char *next = (char*)realloc (marlin_agent_policy_routing_native_scratch, len + 1);
    if (next == NULL) {
      return "";
    }
    marlin_agent_policy_routing_native_scratch = next;
    marlin_agent_policy_routing_native_scratch_capacity = len + 1;
  }
  if (len != 0) {
    memcpy (marlin_agent_policy_routing_native_scratch, value.ptr, len);
  }
  marlin_agent_policy_routing_native_scratch[len] = '\0';
  return marlin_agent_policy_routing_native_scratch;
}

static int
marlin_agent_policy_routing_request_abi_version (MarlinAgentPolicyRoutingRequest *request)
{
  if (request == NULL) {
    return 0;
  }
  return (int)request->abi_version;
}

static char*
marlin_agent_policy_routing_request_graph_id (MarlinAgentPolicyRoutingRequest *request)
{
  if (request == NULL) {
    return "";
  }
  return marlin_agent_policy_routing_utf8_to_c_string (request->graph_id);
}

static char*
marlin_agent_policy_routing_request_policy_scope (MarlinAgentPolicyRoutingRequest *request)
{
  if (request == NULL) {
    return "";
  }
  return marlin_agent_policy_routing_utf8_to_c_string (request->policy_scope);
}

static char*
marlin_agent_policy_routing_request_root_node (MarlinAgentPolicyRoutingRequest *request)
{
  if (request == NULL) {
    return "";
  }
  return marlin_agent_policy_routing_utf8_to_c_string (request->root_node);
}

static int
marlin_agent_policy_routing_request_candidate_edges_len (MarlinAgentPolicyRoutingRequest *request)
{
  if (request == NULL) {
    return 0;
  }
  return (int)request->candidate_edges.len;
}

static char*
marlin_agent_policy_routing_request_candidate_edge_at (MarlinAgentPolicyRoutingRequest *request,
                                                       int index)
{
  if (request == NULL || request->candidate_edges.items == NULL ||
      index < 0 || ((size_t)index) >= request->candidate_edges.len) {
    return "";
  }
  return marlin_agent_policy_routing_utf8_to_c_string (request->candidate_edges.items[index]);
}

static int
marlin_agent_policy_routing_request_routing_evidence_len (MarlinAgentPolicyRoutingRequest *request)
{
  if (request == NULL) {
    return 0;
  }
  return (int)request->routing_evidence.len;
}

static char*
marlin_agent_policy_routing_request_routing_evidence_kind_at (MarlinAgentPolicyRoutingRequest *request,
                                                              int index)
{
  if (request == NULL || request->routing_evidence.items == NULL ||
      index < 0 || ((size_t)index) >= request->routing_evidence.len) {
    return "";
  }
  return marlin_agent_policy_routing_utf8_to_c_string (
    request->routing_evidence.items[index].evidence_kind);
}

static char*
marlin_agent_policy_routing_request_routing_evidence_id_at (MarlinAgentPolicyRoutingRequest *request,
                                                            int index)
{
  if (request == NULL || request->routing_evidence.items == NULL ||
      index < 0 || ((size_t)index) >= request->routing_evidence.len) {
    return "";
  }
  return marlin_agent_policy_routing_utf8_to_c_string (
    request->routing_evidence.items[index].evidence_id);
}

static int
marlin_agent_policy_routing_set_projection (MarlinAgentPolicyRoutingProjection *out,
                                            char *routing_decision)
{
  size_t len;
  char *next;

  if (out == NULL) {
    return MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER;
  }
  if (routing_decision == NULL) {
    return MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION;
  }

  len = strlen (routing_decision);
  if (len + 1 > marlin_agent_policy_routing_native_decision_capacity) {
    next = (char*)realloc (marlin_agent_policy_routing_native_decision, len + 1);
    if (next == NULL) {
      return MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION;
    }
    marlin_agent_policy_routing_native_decision = next;
    marlin_agent_policy_routing_native_decision_capacity = len + 1;
  }

  memcpy (marlin_agent_policy_routing_native_decision, routing_decision, len + 1);
  out->abi_version = MARLIN_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION;
  out->routing_decision.ptr = (const uint8_t*)marlin_agent_policy_routing_native_decision;
  out->routing_decision.len = len;
  return MARLIN_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK;
}
END-C
)

  (c-define-type agent-policy-routing-request "MarlinAgentPolicyRoutingRequest")
  (c-define-type agent-policy-routing-request*
    (pointer agent-policy-routing-request (agent-policy-routing-request*)))
  (c-define-type agent-policy-routing-projection "MarlinAgentPolicyRoutingProjection")
  (c-define-type agent-policy-routing-projection*
    (pointer agent-policy-routing-projection (agent-policy-routing-projection*)))

  (define-c-lambda native-request-abi-version (agent-policy-routing-request*) int
    "marlin_agent_policy_routing_request_abi_version")
  (define-c-lambda native-request-graph-id (agent-policy-routing-request*) char-string
    "marlin_agent_policy_routing_request_graph_id")
  (define-c-lambda native-request-policy-scope (agent-policy-routing-request*) char-string
    "marlin_agent_policy_routing_request_policy_scope")
  (define-c-lambda native-request-root-node (agent-policy-routing-request*) char-string
    "marlin_agent_policy_routing_request_root_node")
  (define-c-lambda native-request-candidate-edges-len (agent-policy-routing-request*) int
    "marlin_agent_policy_routing_request_candidate_edges_len")
  (define-c-lambda native-request-candidate-edge-at (agent-policy-routing-request* int) char-string
    "marlin_agent_policy_routing_request_candidate_edge_at")
  (define-c-lambda native-request-routing-evidence-len (agent-policy-routing-request*) int
    "marlin_agent_policy_routing_request_routing_evidence_len")
  (define-c-lambda native-request-routing-evidence-kind-at
    (agent-policy-routing-request* int) char-string
    "marlin_agent_policy_routing_request_routing_evidence_kind_at")
  (define-c-lambda native-request-routing-evidence-id-at
    (agent-policy-routing-request* int) char-string
    "marlin_agent_policy_routing_request_routing_evidence_id_at")
  (define-c-lambda native-set-projection!
    (agent-policy-routing-projection* char-string) int
    "marlin_agent_policy_routing_set_projection"))

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
(define (native-request-routing-evidence request)
  (map (lambda (index)
         (make-marlin-agent-policy-routing-evidence
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
      (let ((routing-projection
             (marlin-agent-policy-routing-select-edges
              (native-request-graph-id request)
              (native-request-policy-scope request)
              (native-request-root-node request)
              (native-request-candidate-edges request)
              (native-request-routing-evidence request))))
        (native-set-projection!
         projection
         (marlin-agent-policy-routing-projection-decision routing-projection))))))
