;;; -*- Gerbil -*-
;;; Gerbil native C ABI for the Marlin Deck runtime model-route selector.

(import :std/foreign)

(declare
  (block)
  (standard-bindings)
  (extended-bindings)
  (not safe))

(begin-ffi (native-request-abi-version
            native-request-command
            native-request-agent-scope
            native-request-policies-len
            native-request-policy-at
            native-policy-name
            native-policy-provider
            native-policy-model
            native-policy-context-mode
            native-policy-isolation-mode
            native-policy-command-prefixes-len
            native-policy-command-prefix-at
            native-policy-agent-scopes-len
            native-policy-agent-scope-at
            native-set-selection!)
  (c-declare #<<END-C
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

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

static char *marlin_deck_runtime_native_scratch = NULL;
static size_t marlin_deck_runtime_native_scratch_capacity = 0;
static int marlin_deck_runtime_native_initialized = 0;

___BEGIN_NEW_LNK
___DEF_NEW_LNK(___LNK_deck_2d_runtime_2d_native_7e_0__)
___END_NEW_LNK

int
marlin_deck_runtime_initialize (void)
{
  ___SCMOBJ setup_result;
  ___setup_params_struct setup_params;

  if (marlin_deck_runtime_native_initialized != 0) {
    return 0;
  }

  ___setup_params_reset (&setup_params);
  setup_params.version = ___VERSION;
  setup_params.linker = ___LNK_deck_2d_runtime_2d_native_7e_0__;
  setup_result = ___setup (&setup_params);
  if (setup_result != ___FIX(___NO_ERR)) {
    return ___INT(setup_result);
  }

  marlin_deck_runtime_native_initialized = 1;
  return 0;
}

static char*
marlin_deck_runtime_utf8_to_c_string (MarlinDeckRuntimeUtf8 value)
{
  size_t len = (size_t)value.len;
  if (value.ptr == NULL && len != 0) {
    return "";
  }
  if (len + 1 > marlin_deck_runtime_native_scratch_capacity) {
    char *next = (char*)realloc (marlin_deck_runtime_native_scratch, len + 1);
    if (next == NULL) {
      return "";
    }
    marlin_deck_runtime_native_scratch = next;
    marlin_deck_runtime_native_scratch_capacity = len + 1;
  }
  if (len != 0) {
    memcpy (marlin_deck_runtime_native_scratch, value.ptr, len);
  }
  marlin_deck_runtime_native_scratch[len] = '\0';
  return marlin_deck_runtime_native_scratch;
}

static int
marlin_deck_runtime_request_abi_version (MarlinDeckRuntimeModelRouteRequest *request)
{
  if (request == NULL) {
    return 0;
  }
  return (int)request->abi_version;
}

static char*
marlin_deck_runtime_request_command (MarlinDeckRuntimeModelRouteRequest *request)
{
  if (request == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (request->command);
}

static char*
marlin_deck_runtime_request_agent_scope (MarlinDeckRuntimeModelRouteRequest *request)
{
  if (request == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (request->agent_scope);
}

static int
marlin_deck_runtime_request_policies_len (MarlinDeckRuntimeModelRouteRequest *request)
{
  if (request == NULL) {
    return 0;
  }
  return (int)request->policies_len;
}

static MarlinDeckRuntimeModelRoutePolicy*
marlin_deck_runtime_request_policy_at (MarlinDeckRuntimeModelRouteRequest *request,
                                       int index)
{
  if (request == NULL || request->policies == NULL ||
      index < 0 || ((uintptr_t)index) >= request->policies_len) {
    return NULL;
  }
  return (MarlinDeckRuntimeModelRoutePolicy*)&request->policies[index];
}

static char*
marlin_deck_runtime_policy_name (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->name);
}

static char*
marlin_deck_runtime_policy_provider (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->provider);
}

static char*
marlin_deck_runtime_policy_model (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->model);
}

static char*
marlin_deck_runtime_policy_context_mode (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->context_mode);
}

static char*
marlin_deck_runtime_policy_isolation_mode (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->isolation_mode);
}

static int
marlin_deck_runtime_policy_command_prefixes_len (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return 0;
  }
  return (int)policy->command_prefixes.len;
}

static char*
marlin_deck_runtime_policy_command_prefix_at (MarlinDeckRuntimeModelRoutePolicy *policy,
                                              int index)
{
  if (policy == NULL || policy->command_prefixes.items == NULL ||
      index < 0 || ((uintptr_t)index) >= policy->command_prefixes.len) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->command_prefixes.items[index]);
}

static int
marlin_deck_runtime_policy_agent_scopes_len (MarlinDeckRuntimeModelRoutePolicy *policy)
{
  if (policy == NULL) {
    return 0;
  }
  return (int)policy->agent_scopes.len;
}

static char*
marlin_deck_runtime_policy_agent_scope_at (MarlinDeckRuntimeModelRoutePolicy *policy,
                                           int index)
{
  if (policy == NULL || policy->agent_scopes.items == NULL ||
      index < 0 || ((uintptr_t)index) >= policy->agent_scopes.len) {
    return "";
  }
  return marlin_deck_runtime_utf8_to_c_string (policy->agent_scopes.items[index]);
}

static int
marlin_deck_runtime_set_selection (MarlinDeckRuntimeModelRouteSelection *out,
                                   int matched,
                                   int policy_index)
{
  if (out == NULL) {
    return MARLIN_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER;
  }
  if (matched != 0 && policy_index < 0) {
    return MARLIN_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION;
  }
  out->abi_version = MARLIN_DECK_RUNTIME_NATIVE_ABI_VERSION;
  out->matched = matched != 0 ? 1u : 0u;
  out->reserved[0] = 0u;
  out->reserved[1] = 0u;
  out->reserved[2] = 0u;
  out->policy_index = matched != 0 ? (uintptr_t)policy_index : MARLIN_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX;
  return MARLIN_DECK_RUNTIME_NATIVE_STATUS_OK;
}
END-C
)

  (c-define-type deck-runtime-request "MarlinDeckRuntimeModelRouteRequest")
  (c-define-type deck-runtime-request*
    (pointer deck-runtime-request (deck-runtime-request*)))
  (c-define-type deck-runtime-policy "MarlinDeckRuntimeModelRoutePolicy")
  (c-define-type deck-runtime-policy*
    (pointer deck-runtime-policy (deck-runtime-policy*)))
  (c-define-type deck-runtime-selection "MarlinDeckRuntimeModelRouteSelection")
  (c-define-type deck-runtime-selection*
    (pointer deck-runtime-selection (deck-runtime-selection*)))

  (define-c-lambda native-request-abi-version (deck-runtime-request*) int
    "marlin_deck_runtime_request_abi_version")
  (define-c-lambda native-request-command (deck-runtime-request*) char-string
    "marlin_deck_runtime_request_command")
  (define-c-lambda native-request-agent-scope (deck-runtime-request*) char-string
    "marlin_deck_runtime_request_agent_scope")
  (define-c-lambda native-request-policies-len (deck-runtime-request*) int
    "marlin_deck_runtime_request_policies_len")
  (define-c-lambda native-request-policy-at (deck-runtime-request* int) deck-runtime-policy*
    "marlin_deck_runtime_request_policy_at")
  (define-c-lambda native-policy-name (deck-runtime-policy*) char-string
    "marlin_deck_runtime_policy_name")
  (define-c-lambda native-policy-provider (deck-runtime-policy*) char-string
    "marlin_deck_runtime_policy_provider")
  (define-c-lambda native-policy-model (deck-runtime-policy*) char-string
    "marlin_deck_runtime_policy_model")
  (define-c-lambda native-policy-context-mode (deck-runtime-policy*) char-string
    "marlin_deck_runtime_policy_context_mode")
  (define-c-lambda native-policy-isolation-mode (deck-runtime-policy*) char-string
    "marlin_deck_runtime_policy_isolation_mode")
  (define-c-lambda native-policy-command-prefixes-len (deck-runtime-policy*) int
    "marlin_deck_runtime_policy_command_prefixes_len")
  (define-c-lambda native-policy-command-prefix-at (deck-runtime-policy* int) char-string
    "marlin_deck_runtime_policy_command_prefix_at")
  (define-c-lambda native-policy-agent-scopes-len (deck-runtime-policy*) int
    "marlin_deck_runtime_policy_agent_scopes_len")
  (define-c-lambda native-policy-agent-scope-at (deck-runtime-policy* int) char-string
    "marlin_deck_runtime_policy_agent_scope_at")
  (define-c-lambda native-set-selection! (deck-runtime-selection* int int) int
    "marlin_deck_runtime_set_selection"))

(define marlin-deck-runtime-native-abi-version 1)
(define marlin-deck-runtime-native-status-abi-mismatch 3)

(define (native-string-list len ref)
  (let loop ((index 0) (acc '()))
    (if (= index len)
      (reverse acc)
      (loop (+ index 1) (cons (ref index) acc)))))

(define (native-policy-command-prefixes policy)
  (native-string-list
   (native-policy-command-prefixes-len policy)
   (lambda (index) (native-policy-command-prefix-at policy index))))

(define (native-policy-agent-scopes policy)
  (native-string-list
   (native-policy-agent-scopes-len policy)
   (lambda (index) (native-policy-agent-scope-at policy index))))

(define (native-policy->policy policy)
  (list (native-policy-name policy)
        (native-policy-provider policy)
        (native-policy-model policy)
        (native-policy-command-prefixes policy)
        (native-policy-agent-scopes policy)
        (native-policy-context-mode policy)
        (native-policy-isolation-mode policy)))

(define (native-request-policies request)
  (let loop ((index 0)
             (len (native-request-policies-len request))
             (acc '()))
    (if (= index len)
      (reverse acc)
      (loop (+ index 1)
            len
            (cons (native-policy->policy
                   (native-request-policy-at request index))
                  acc)))))

(define (policy-name policy) (list-ref policy 0))
(define (policy-provider policy) (list-ref policy 1))
(define (policy-model policy) (list-ref policy 2))
(define (policy-command-prefixes policy) (list-ref policy 3))
(define (policy-agent-scopes policy) (list-ref policy 4))
(define (policy-context-mode policy) (list-ref policy 5))
(define (policy-isolation-mode policy) (list-ref policy 6))

(define (string-prefix? prefix value)
  (let ((prefix-length (string-length prefix))
        (value-length (string-length value)))
    (if (> prefix-length value-length)
      #f
      (let loop ((index 0))
        (cond
          ((= index prefix-length) #t)
          ((char=? (string-ref prefix index) (string-ref value index))
           (loop (+ index 1)))
          (else #f))))))

(define (string-member? value values)
  (if (member value values) #t #f))

(define (any-string-prefix? prefixes value)
  (let loop ((remaining prefixes))
    (cond
      ((null? remaining) #f)
      ((string-prefix? (car remaining) value) #t)
      (else (loop (cdr remaining))))))

(define (policy-match? policy command agent-scope)
  (and (any-string-prefix? (policy-command-prefixes policy) command)
       (string-member? agent-scope (policy-agent-scopes policy))))

(define (select-policy-index policies command agent-scope)
  (let loop ((remaining policies) (index 0))
    (cond
      ((null? remaining) #f)
      ((policy-match? (car remaining) command agent-scope)
       index)
      (else (loop (cdr remaining) (+ index 1))))))

(extern marlin-deck-runtime-select-model-route)
(begin-foreign
  (namespace ("marlin-deck-runtime/src/marlin/deck-runtime-native#"
              marlin-deck-runtime-native-abi-version
              native-request-policies
              native-request-command
              native-request-agent-scope
              select-policy-index
              native-set-selection!))

  (c-define (marlin-deck-runtime-select-model-route request selection)
    (deck-runtime-request* deck-runtime-selection*) int
    "marlin_deck_runtime_select_model_route" ""
    (if (not (= (native-request-abi-version request)
                marlin-deck-runtime-native-abi-version))
      marlin-deck-runtime-native-status-abi-mismatch
      (let ((policy-index
             (select-policy-index
              (native-request-policies request)
              (native-request-command request)
              (native-request-agent-scope request))))
        (native-set-selection!
         selection
         (if policy-index 1 0)
         (if policy-index policy-index -1))))))
