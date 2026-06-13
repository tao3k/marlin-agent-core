#ifndef MARLIN_DECK_RUNTIME_NATIVE_H
#define MARLIN_DECK_RUNTIME_NATIVE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define MARLIN_DECK_RUNTIME_NATIVE_ABI_VERSION 1u
#define MARLIN_DECK_RUNTIME_NATIVE_STATUS_OK 0
#define MARLIN_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER 2
#define MARLIN_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH 3
#define MARLIN_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION 4
#define MARLIN_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX UINTPTR_MAX

typedef struct MarlinDeckRuntimeUtf8 {
  const uint8_t *ptr;
  uintptr_t len;
} MarlinDeckRuntimeUtf8;

typedef struct MarlinDeckRuntimeUtf8List {
  const MarlinDeckRuntimeUtf8 *items;
  uintptr_t len;
} MarlinDeckRuntimeUtf8List;

typedef struct MarlinDeckRuntimeModelRoutePolicy {
  MarlinDeckRuntimeUtf8 name;
  MarlinDeckRuntimeUtf8 provider;
  MarlinDeckRuntimeUtf8 model;
  MarlinDeckRuntimeUtf8List command_prefixes;
  MarlinDeckRuntimeUtf8List agent_scopes;
  MarlinDeckRuntimeUtf8 context_mode;
  MarlinDeckRuntimeUtf8 isolation_mode;
} MarlinDeckRuntimeModelRoutePolicy;

typedef struct MarlinDeckRuntimeModelRouteRequest {
  uint32_t abi_version;
  MarlinDeckRuntimeUtf8 command;
  MarlinDeckRuntimeUtf8 agent_scope;
  const MarlinDeckRuntimeModelRoutePolicy *policies;
  uintptr_t policies_len;
} MarlinDeckRuntimeModelRouteRequest;

typedef struct MarlinDeckRuntimeModelRouteSelection {
  uint32_t abi_version;
  uint8_t matched;
  uint8_t reserved[3];
  uintptr_t policy_index;
} MarlinDeckRuntimeModelRouteSelection;

typedef int32_t MarlinDeckRuntimeStatus;
typedef int32_t MarlinDeckRuntimeInitializeStatus;

MarlinDeckRuntimeInitializeStatus marlin_deck_runtime_initialize(void);

MarlinDeckRuntimeStatus marlin_deck_runtime_select_model_route(
    const MarlinDeckRuntimeModelRouteRequest *request,
    MarlinDeckRuntimeModelRouteSelection *selection);

typedef MarlinDeckRuntimeStatus (*MarlinDeckRuntimeSelectModelRouteFn)(
    const MarlinDeckRuntimeModelRouteRequest *request,
    MarlinDeckRuntimeModelRouteSelection *selection);

typedef MarlinDeckRuntimeInitializeStatus (*MarlinDeckRuntimeInitializeFn)(
    void);

#ifdef __cplusplus
}
#endif

#endif
