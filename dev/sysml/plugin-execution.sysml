// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Target execution path. Org counterpart: 40.166 Plugin Contract v0, #plugin-request-path.
package MarlinPluginExecution {
    private import ScalarValues::*;
    private import MarlinPluginEngine::*;

    action def <'MAR-PLG-P01'> PluginRequestPath {
        doc /* One plugin request, with a compiled Rust path and an optional Scheme
               planning path. The decision outputs stand for host observations.
               The lifecycle model describes cancellation and terminal states. */

        action decodePluginRequest {
            doc /* The Rust gRPC service decodes one command into a typed
                   request. Admission starts as soon as the request is ready. */
        }
        action bindAndAdmitRequest {
            out queued : Boolean;
            doc /* Rust binds project, generation, policy epoch, and request
                   identity. It checks authority and reserves channel capacity. */
        }
        action drainReadyMessages {
            doc /* Tokio schedules Rust work. The Rust message owner may drain
                   a bounded batch of ready messages, preserving scope order.
                   Dispatch can start with one ready item. */
        }
        action resolvePolicyPlan {
            out needsSchemeDecision : Boolean;
            doc /* Rust selects a compiled plan for the current policy epoch
                   and input dependencies, or asks Scheme for a decision. */
        }
        action assembleSchemeBatch {
            doc /* Rust groups ready requests that share a Scheme VM owner,
                   project scope, generation, epoch, and ABI operation.
                   Each request has all its inputs. Count, bytes,
                   work estimate, and remaining wait budget bound the batch.
                   Reserve bounded result capacity before admitting Scheme work. */
        }
        action invokeSchemeCAbi {
            doc /* One typed C ABI submission per batch. On the Scheme VM
                   owner, the bridge constructs rooted Scheme records and
                   vectors from owned Rust fields and calls the batch export.
                   The Context adapter
                   transfers shared scope fields once per batch and projects
                   owned digests in submission order. */
        }
        action evaluateWithSchemeActors {
            doc /* Gerbil Actor and green thread scheduling runs inside the
                   owner-affine VM. Internal Actor messages, heap values, and
                   continuations stay with their Scheme owner. */
        }
        action retireSchemeWork {
            doc /* The bridge projects owned results to reserved host space.
                   The owner releases VM capacity after retirement.
                   Caller consumption has a separate lifetime and budget. */
        }
        action qualifyOwnedProjection {
            out usable : Boolean;
            doc /* Rust correlates per-request results from the batch, checks
                   the ABI and schema, and rechecks current generation and
                   epoch before using the owned projection. */
        }
        action executeRustPlan {
            doc /* Tokio drives compiled transitions. Rust checks live grants
                   before each effect and records each node and effect outcome.
                   Independent CPU work on qualified, owned Rust data may use
                   Rayon. Effect and receipt ordering follows the plan. */
        }
        action recordRejection {
            doc /* Rust retains the applicable rejection. Rejected or stale
                   requests confer zero authority for new effects. */
        }
        action finalizeExecutionReceipt {
            doc /* Rust binds terminal outcome to its request and evidence.
                   Each semantic receipt keeps its identity and ordering. */
        }
        action returnRequestOutcome {
            doc /* Rust returns the typed outcome or receipt reference. */
        }
        action offerClientEvent {
            doc /* Rust offers a scoped event to a bounded async channel.
                   Delivery may group events while preserving stream order. */
        }

        decide callAdmission;
        decide policyRoute;
        decide projectionDisposition;
        merge rejected;
        merge readyPlan;
        merge terminalOutcome;
        fork publishOutcome;

        first start then decodePluginRequest;
        first decodePluginRequest then bindAndAdmitRequest;
        first bindAndAdmitRequest then callAdmission;
        first callAdmission if bindAndAdmitRequest.queued then drainReadyMessages;
        first callAdmission if not bindAndAdmitRequest.queued then rejected;
        first drainReadyMessages then resolvePolicyPlan;
        first resolvePolicyPlan then policyRoute;
        first policyRoute if not resolvePolicyPlan.needsSchemeDecision then readyPlan;
        first policyRoute if resolvePolicyPlan.needsSchemeDecision then assembleSchemeBatch;
        first assembleSchemeBatch then invokeSchemeCAbi;
        first invokeSchemeCAbi then evaluateWithSchemeActors;
        first evaluateWithSchemeActors then retireSchemeWork;
        first retireSchemeWork then qualifyOwnedProjection;
        first qualifyOwnedProjection then projectionDisposition;
        first projectionDisposition if qualifyOwnedProjection.usable then readyPlan;
        first projectionDisposition if not qualifyOwnedProjection.usable then rejected;
        first rejected then recordRejection;
        first readyPlan then executeRustPlan;
        first recordRejection then terminalOutcome;
        first executeRustPlan then terminalOutcome;
        first terminalOutcome then finalizeExecutionReceipt;
        first finalizeExecutionReceipt then publishOutcome;
        first publishOutcome then returnRequestOutcome;
        first publishOutcome then offerClientEvent;
    }

    action pluginRequest : PluginRequestPath;

    allocation ingressOwner allocate pluginRequest.decodePluginRequest to target::pluginEngine.grpcService;
    allocation admissionOwner allocate pluginRequest.bindAndAdmitRequest to target::pluginEngine.activationGate;
    allocation readyQueueOwner allocate pluginRequest.drainReadyMessages to target::pluginEngine.runtime.messageChannels;
    allocation planOwner allocate pluginRequest.resolvePolicyPlan to target::pluginEngine.runtime.compiledExecution;
    allocation batchOwner allocate pluginRequest.assembleSchemeBatch to target::pluginEngine.runtime.messageChannels;
    allocation schemeEntryOwner allocate pluginRequest.invokeSchemeCAbi to target::pluginEngine.schemeCAbiBridge;
    allocation schemeEvaluationOwner allocate pluginRequest.evaluateWithSchemeActors to target::schemeVm;
    allocation retirementOwner allocate pluginRequest.retireSchemeWork to target::pluginEngine.schemeCAbiBridge;
    allocation projectionOwner allocate pluginRequest.qualifyOwnedProjection to target::pluginEngine.activationGate;
    allocation executionOwner allocate pluginRequest.executeRustPlan to target::pluginEngine.runtime.compiledExecution;
    allocation rejectionOwner allocate pluginRequest.recordRejection to target::pluginEngine.evidence;
    allocation receiptOwner allocate pluginRequest.finalizeExecutionReceipt to target::pluginEngine.evidence;
    allocation replyOwner allocate pluginRequest.returnRequestOutcome to target::pluginEngine.grpcService;
    allocation eventQueueOwner allocate pluginRequest.offerClientEvent to target::pluginEngine.runtime.messageChannels;
}
