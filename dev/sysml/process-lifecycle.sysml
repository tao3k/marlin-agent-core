// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinProcessLifecycle {
    doc /* Ownership and verification of optional quota for managed direct children.
           Unix containment is limited to the managed process group; this does
           not claim containment of escaped descendants or eventual cleanup. */
    part <'MAR-PROC-A01'> managedProcess {
        doc /* Rust: crates/marlin-agent-runtime/src/process/managed.rs;
               quota configuration and sharing: tokio_runtime/handle.rs and context.rs. */
    }
    part <'MAR-PROC-A03'> toolProcessFacade {
        doc /* Kernel request and command template propagate the Rust containment
               requirement to the managed-process admission boundary. */
    }
    part <'MAR-PROC-A04'> strictTreeCandidate {
        doc /* Proposed Linux cgroup-v2 containment; no production backend is
               enabled. Source evidence: strict-tree-proposal.qnt and the
               Linux OS qualification fixture. Windows and macOS need separate
               platform-specific qualification. */
    }
    part <'MAR-PROC-A02'> lifecycleVerification {
        doc /* Quint: dev/quint/process-quota.qnt; executable OS observations:
               crates/marlin-agent-runtime/tests/unit/runtime/process/quota_trace.rs;
               process-tree.qnt and quota_tree_trace.rs observe group and pipe states. */
    }
    part <'MAR-PROC-A05'> boundedProcessContract {
        doc /* Lower-level independent Rust contract: crates/marlin-agent-protocol/src/process.rs;
               Runtime adapter: crates/marlin-agent-runtime/src/process_executor.rs.
               Git constructs read-only commands but does not own the Runtime. */
    }
    part <'MAR-PROC-A06'> processWatchdog {
        doc /* Monotonic host heartbeats, process progress, and the command
               deadline provide distinct observations of a Runtime resource.
               The observer has no process handle. */
    }
    requirement <'MAR-PROC-R17'> backgroundProcessHealth {
        doc /* A missed host heartbeat and absent task progress have independent
               clocks. Report each once per period of silence, and rearm it on
               fresh evidence. Both are advisory even for the exact generation.
               An explicit command deadline can authorize owner cleanup. */
    }
    allocation classifyBackgroundHealth allocate backgroundProcessHealth to processWatchdog;
    allocation verifyBackgroundHealth allocate backgroundProcessHealth to lifecycleVerification;
    part <'MAR-PROC-A07'> hostObserver {
        doc /* A separate macOS process receives bounded host progress pulses
               and observes the host exit through Darwin EVFILT_PROC.
               The implementation belongs to marlin-agent-telemetry. */
    }
    requirement <'MAR-PROC-R18'> independentHostObservation {
        doc /* TUI and Tokio heartbeat silence, channel failure, and kernel
               host exit are distinct evidence. Task idle is process-scoped.
               The observer emits evidence and has no Runtime cleanup handle.
               Export or observer failure must not prevent Runtime cleanup. */
    }
    allocation observeHostIndependently allocate independentHostObservation to hostObserver;
    allocation verifyHostIndependently allocate independentHostObservation to lifecycleVerification;
    requirement <'MAR-PROC-R19'> burstAdmission {
        doc /* Limit concurrent managed process admissions and reserve declared
               memory against one shared budget before launching. Release the
               reservation with the existing process and pipe lease. macOS
               footprint samples arrive from the telemetry sampler and inform
               admission. Runtime retains the host baseline plus the greater
               of observed child footprint and outstanding child reservations. */
    }
    allocation boundBurst allocate burstAdmission to managedProcess;
    allocation verifyBurst allocate burstAdmission to lifecycleVerification;
    requirement <'MAR-PROC-R15'> watchdogObservationOnly {
        doc /* An elapsed observation deadline does not change process state,
               signal a process, or release its resource reservation. */
    }
    requirement <'MAR-PROC-R16'> watchdogCleanupAuthority {
        doc /* Only the managed owner may act on a watchdog observation. It
               compares session, operation, and generation with its owned
               handle. A mismatch returns the handle without side effects.
               A match uses the existing process-group and pipe reaper. */
    }
    allocation observeDeadline allocate watchdogObservationOnly to processWatchdog;
    allocation applyObservation allocate watchdogCleanupAuthority to managedProcess;
    allocation watchdogObservationEvidence allocate watchdogObservationOnly to lifecycleVerification;
    allocation watchdogCleanupEvidence allocate watchdogCleanupAuthority to lifecycleVerification;
    requirement <'MAR-PROC-R14'> trackedProcessCleanup {
        doc /* A managed child reserves a Runtime resource before spawning.
               Its reservation follows the child through reaping and stays
               held by each exported pipe. Shutdown waits for the reservation. */
    }
    requirement <'MAR-PROC-R11'> genericBoundedExecution {
        doc /* A caller injects a generic bounded process executor. Its output
               limit and deadline apply independently of the tool's identity. */
    }
    requirement <'MAR-PROC-R12'> atomicShutdownAdmission {
        doc /* Closing and admission share one linearization point. Admitted
               work contributes to the drain. A rejected task runs no body,
               and a rejected process launches no executable. */
    }
    requirement <'MAR-PROC-R13'> resourceIdentity {
        doc /* A typed session, operation and Runtime-allocated generation
               identify each resource in this host process. Reserve a managed
               operation within its session before launching it. Cleanup of
               a previous generation cannot modify its replacement. */
    }
    requirement <'MAR-PROC-R01'> quotaHeldUntilExit {
        doc /* When enabled, each permit belongs to one managed direct child.
               A cancelled task may complete while that child drains. Admission
               must retain its permit until the child's exit is confirmed. */
    }
    requirement <'MAR-PROC-R02'> independentEvidence {
        doc /* Replay generated lifecycle traces through the Rust process entry;
               compare direct-child OS observations and admission at each step.
               A premature-release mutation must be detected by this replay. */
    }
    requirement <'MAR-PROC-R03'> processGroupDrain {
        doc /* On Unix, retain the operation reservation and any configured
               permit after leader exit until its process group is observed
               absent. Escaped processes and group identity reuse are not
               resolved by this mechanism. */
    }
    requirement <'MAR-PROC-R04'> managedPipeDrain {
        doc /* Successful output collection retains the permit until stdout and
               stderr reach EOF and owned read ends close. Cancellation-order
               closure is protected by leased endpoints. Unmanaged duplicated
               descriptors and external writers are not tracked. */
    }
    requirement <'MAR-PROC-R05'> strictDescendantAdmission {
        doc /* A request requiring OS-enforced descendant-tree containment must
               be refused before spawning unless such a backend is available.
               Platform-default process groups may be escaped and do not meet it. */
    }
    requirement <'MAR-PROC-R06'> externalManagedPipeLease {
        doc /* A caller-held managed stdin pipe retains quota until that handle is
               closed, even if the process and its group have already exited. */
    }
    requirement <'MAR-PROC-R07'> cancelledPipeOrdering {
        doc /* Runtime-owned output pipe endpoints carry the same quota lease;
               cancellation cannot release it before those endpoints close and
               process-group cleanup is confirmed. */
    }
    requirement <'MAR-PROC-R08'> strictLaunchOrdering {
        doc /* Target code may execute only after placement in the containment
               boundary and verification that it cannot migrate out. Failure or
               cancellation before readiness must not execute target code. */
    }
    requirement <'MAR-PROC-R09'> strictCleanupOrdering {
        doc /* After cancellation, capacity is reusable only after authoritative
               tree-empty observation and closure of every accounted pipe lease. */
    }
    requirement <'MAR-PROC-R10'> strictPlatformReadiness {
        doc /* Platform strict admission remains closed until the actual backend,
               authority model and OS-observed failure controls are qualified. */
    }
    allocation proposalLaunch allocate strictLaunchOrdering to strictTreeCandidate;
    allocation proposalCleanup allocate strictCleanupOrdering to strictTreeCandidate;
    allocation proposalReadiness allocate strictPlatformReadiness to strictTreeCandidate;
    allocation proposalEvidence allocate strictLaunchOrdering to lifecycleVerification;
    allocation proposalCleanupEvidence allocate strictCleanupOrdering to lifecycleVerification;
    allocation strictAdmission allocate strictDescendantAdmission to managedProcess;
    allocation strictFacade allocate strictDescendantAdmission to toolProcessFacade;
    allocation externalLease allocate externalManagedPipeLease to managedProcess;
    allocation cancelOrdering allocate cancelledPipeOrdering to managedProcess;
    allocation strictEvidence allocate strictDescendantAdmission to lifecycleVerification;
    allocation leaseEvidence allocate externalManagedPipeLease to lifecycleVerification;
    allocation orderingEvidence allocate cancelledPipeOrdering to lifecycleVerification;
    allocation groupOwnership allocate processGroupDrain to managedProcess;
    allocation pipeOwnership allocate managedPipeDrain to managedProcess;
    allocation groupEvidence allocate processGroupDrain to lifecycleVerification;
    allocation pipeEvidence allocate managedPipeDrain to lifecycleVerification;
    allocation processOwnership allocate quotaHeldUntilExit to managedProcess;
    allocation evidenceOwnership allocate independentEvidence to lifecycleVerification;
    allocation trackedCleanup allocate trackedProcessCleanup to managedProcess;
    allocation boundedExecution allocate genericBoundedExecution to boundedProcessContract;
    allocation trackedCleanupEvidence allocate trackedProcessCleanup to lifecycleVerification;
    allocation admissionOwnership allocate atomicShutdownAdmission to managedProcess;
    allocation admissionEvidence allocate atomicShutdownAdmission to lifecycleVerification;
    allocation identityOwnership allocate resourceIdentity to managedProcess;
    allocation identityEvidence allocate resourceIdentity to lifecycleVerification;
}
