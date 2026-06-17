#ifndef MARLIN_AGENT_POLICY_ROUTING_NATIVE_H
#define MARLIN_AGENT_POLICY_ROUTING_NATIVE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

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

typedef int32_t MarlinAgentPolicyRoutingStatus;

typedef MarlinAgentPolicyRoutingStatus (*MarlinAgentPolicyRoutingSelectEdgesFn)(
    const MarlinAgentPolicyRoutingRequest *request,
    MarlinAgentPolicyRoutingProjection *projection);

int32_t marlin_agent_policy_routing_initialize(void);

MarlinAgentPolicyRoutingStatus marlin_agent_policy_routing_select_edges(
    const MarlinAgentPolicyRoutingRequest *request,
    MarlinAgentPolicyRoutingProjection *projection);

#ifdef __cplusplus
}
#endif

#endif
