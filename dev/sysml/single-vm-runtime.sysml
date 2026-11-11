// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Single-VM implementation and its qualification obligations.
package MarlinSingleVmRuntime {
    private import MarlinPluginEngine::*;

    part <'MAR-VM-A01'> singleVmRuntime {
        doc /* Implementation in crates/marlin-deck-runtime/src/runtime.
               Local linked Context, batching and shutdown tests have passed.
               Release Scope-admission samples are recorded in the Org companion. */
        part <'MAR-VM-A02'> admission {
            doc /* Tokio mpsc reserves bounded request slots. Each accepted
                   request owns one oneshot result destination. Admission checks
                   payload size, cancellation, deadline and shutdown state. */
        }
        part <'MAR-VM-A03'> owner {
            doc /* One fixed OS thread initializes the linked Gerbil VM with
                   exactly one active processor. Only this owner constructs,
                   calls, projects and releases Scheme roots. */
        }
        part <'MAR-VM-A04'> contextActors {
            doc /* Scheme creates mailbox Actors on first use of a logical
                   bundle, organization, project, worktree, agent and Session.
                   Repeated turns and changed source cuts reuse that Actor.
                   Messages use Gerbil thread-send and thread-receive.
                   Each Actor forces the lazy Lab Scope identity before
                   posting its independently collectible completion. */
        }
        part <'MAR-VM-A05'> retirement {
            doc /* Message IDs, status bytes and successful Scope digests cross
                   as typed vectors. Rust validates IDs and complete packet
                   cardinality before offering any result to a oneshot.
                   Each completed message retires its own native resources.
                   Scope identity does not grant execution authority.
                   Cancellation and deadline are checked again before delivery. */
        }
        part <'MAR-VM-A06'> shutdown {
            doc /* Close admission, drain accepted messages, stop and join
                   identity-owned Actors, release roots and drop the VM.
                   Check Actor and message retirement counters. Publish the typed
                   shutdown receipt after VM retirement and join the OS owner. */
        }
        part <'MAR-VM-A07'> applicationActors {
            doc /* Design for Module-owned Actors. Gerbil spawns and retires
                   Actors according to Module lifecycle and schedules their
                   mailboxes inside the VM. Tokio schedules Rust futures on
                   the host runtime. Actor population is independent of
                   Tokio worker count and Context C ABI transfer size.
                   In-flight collection waits in the Scheme mailbox and
                   permits other green Actors to run. Rust closes a Module's
                   admission before retiring its accepted work. Scheme joins
                   only that Module's Actors and timers. Other Modules remain
                   active. Retirement receipts follow native join and validation. */
        }
        part <'MAR-VM-A08'> linkedRuntimeRoles {
            doc /* The rooted C ABI imports the selected Deck and Context
                   runtime roles into the compiler-owned linked graph.
                   Yuanxu still owns checked publication and Feature selection.
                   Ordinary library imports retain canonical procedure identities.
                   The C ABI checks the selected factory and mailbox factory
                   against the linked constructors before acknowledging startup.
                   The SDK applies SeparateProcedures lowering to the production
                   mailbox module; the benchmark also declares its driver. */
        }
        part <'MAR-VM-A09'> actorPopulation {
            doc /* All live logical owners, including suspended Actors. The
                   identity registry observes population, not runnable count.
                   Module lifecycle determines retirement; no fixed worker pool
                   or in-flight message limit sets the Actor population. */
        }
        part <'MAR-VM-A10'> schedulerReadiness {
            doc /* Gerbil/Gambit owns runnable, running and waiting threads.
                   A wake makes an Actor runnable; scheduler selection determines
                   execution. Publishing a message need not switch threads.
                   Rust neither mirrors nor polls these scheduler queues. */
        }
        part <'MAR-VM-A11'> messageCredits {
            doc /* Rust bounds native in-flight requests independently of live
                   Actors and readiness. A result collected and correlated to
                   its ID returns that message's credit. A waiting Actor retains
                   its identity without retaining completed message credit.
                   Client outcome and VM completion remain separate lifetimes. */
        }
        part <'MAR-VM-A12'> moduleCatalog {
            doc /* Rust mints opaque Module instance identities for registered
                   Context factories. Admission and retirement publication share
                   one short per-Module lock; no lock crosses await or VM entry.
                   Actor keys are scoped by Module. Removed Modules release their
                   registry roots and recent-route cache without tombstones. */
        }
        part <'MAR-VM-A13'> hostWake {
            doc /* A nonblocking Rust-owned pipe enters a Gambit native port.
                   One ingress Actor forwards coalesced host wake notices to the
                   existing owner mailbox. A full pipe already carries a wake.
                   The owner waits in Scheme even when Rust has no work, allowing
                   native I/O, green Actors and timers to advance. No polling
                   clock, second readiness queue or custom Scheme scheduler. */
        }
        part <'MAR-VM-A14'> moduleTimers {
            doc /* Module-owned timer observations use Gambit's timed mailbox
                   wait and monotonic time. Expiry publishes a typed timer ID,
                   Module ID, due time and observed time. Rust validates the
                   complete event window against pending ownership. Retirement
                   sends stop, joins timers and rejects remaining observations.
                   Lab owns temporal policy deadlines and durable Agenda;
                   timer observations grant no external effect authority. */
        }
        part <'MAR-VM-A15'> contextCosts {
            doc /* Measure Rust input projection, Scheme submission, waiting and
                   result projection on the same owner turn, correlated with GC.
                   A component diagnostic separately exercises Lab validation,
                   Scope freezing, digest forcing and result encoding. Reuse
                   Lab's canonical admission Function descriptor across Module
                   roles. Keep runtime accounting and physical allocation-site
                   census distinct; report unobserved RTS allocations explicitly. */
        }
        part <'MAR-VM-A16'> publicationState {
            doc /* Gerbil and Yuanxu own declaration publication, compiler
                   contexts, native Type identities and source-currentness
                   support. This control state has a separate lifetime from
                   business Actors and their message credits. Module retirement
                   joins owned work without discarding compiler authority.
                   Read-only retention diagnostics follow phase and import/export
                   edges to locate code, bindings and evaluation promises beyond
                   the registry's direct entries. Edge visits are not unique
                   object counts or physical bytes. Removing compiler roots in
                   a disposable probe grants no production path. */
        }
        dependency costObservesPublication from contextCosts to publicationState;
        dependency moduleOwnsActors from moduleCatalog to contextActors;
        dependency moduleOwnsTimers from moduleCatalog to moduleTimers;
        dependency ownerWaitsInScheme from owner to hostWake;
        dependency timerWakesOwner from moduleTimers to hostWake;
        dependency contextUsesLibrary from contextActors to contextCosts;
        dependency actorIdentityLifetime from contextActors to actorPopulation;
        dependency actorSchedulingOwner from contextActors to schedulerReadiness;
        dependency messageCreditReturn from retirement to messageCredits;
        dependency ownerUsesLinkedRoles from owner to linkedRuntimeRoles;
        dependency ownerUsesContextActors from owner to contextActors;
        dependency ownerUsesRetirement from owner to retirement;
        dependency admissionUsesOwner from admission to owner;
        dependency shutdownUsesOwner from shutdown to owner;
    }

    requirement <'MAR-VM-R01'> singleVmOwnership {
        doc /* One process-global linked VM and one active Scheme processor.
               No live root or continuation crosses the Rust owner thread. */
    }
    requirement <'MAR-VM-R02'> boundedReadyBatching {
        doc /* Drain ready work without a fill timer. Owner-held request count
               is bounded by admission capacity. Transfer at most 64 Context
               rows and 32 KiB of text per C ABI submission. Independent
               completion windows release message credit and permit refill.
               Actor population follows logical identities. */
    }
    requirement <'MAR-VM-R03'> terminalAccounting {
        doc /* On drained shutdown, received requests equal succeeded,
               cancelled, expired and failed owner outcomes. Actor completion
               and client consumption have distinct lifetimes. */
    }
    requirement <'MAR-VM-R04'> measuredExecution {
        doc /* The crate-owned single_vm_actor bench measures the whole typed
               admission path on one VM. Batch 1 and batch 64 run in separate
               processes with the same Context scopes and
               workload. Use at least four Tokio workers in the test host.
               Record throughput, latency percentiles and actual batch counters. */
    }
    requirement <'MAR-VM-R05'> messageLatency {
        doc /* The mailbox qualification profile offers one million planned
               messages per second and requires planned-to-consumed p99 below
               20 ms. Count every offered message and verify its result.
               Preserve source debt in the latency and retain failed runs.
               Declare payload, handler, identity population and clock scope.
               Context business admission has its own measured workload. */
    }
    requirement <'MAR-VM-R06'> linkedMailboxCompilation {
        doc /* Check the production mailbox identity and declared procedure
               layout in the SDK compiler manifest and compare actual selected
               factory procedures with linked constructors at startup. Reject
               substitutions. Manifest presence alone cannot qualify the reached
               production path; constructor-site evidence must identify reached
               canonical units. A separate benchmark is insufficient. */
    }
    attribute def MessageAvailable;
    attribute def ReceiveWouldBlock;
    attribute def SelectedByScheduler;
    attribute def QuantumExpired;
    attribute def HandlerStopped;

    state def <'MAR-VM-L01'> MailboxScheduling {
        doc /* Descriptive runtime protocol, not a second Marlin scheduler.
               ReceiveWouldBlock follows the native locked queue recheck.
               MessageAvailable wakes a waiter without promising a switch. */
        first start then runnable;
        state runnable;
        state running;
        state waiting;
        state retired;
        transition schedule first runnable accept SelectedByScheduler then running;
        transition awaitMessage first running accept ReceiveWouldBlock then waiting;
        transition wake first waiting accept MessageAvailable then runnable;
        transition keepRunning first running accept MessageAvailable then running;
        transition preempt first running accept QuantumExpired then runnable;
        transition stop first running accept HandlerStopped then retired;
    }
    requirement <'MAR-VM-R07'> independentPopulationAndCredit {
        doc /* Distinguish live Actor population, scheduler readiness and business
               in-flight message credit. Neither a transport window nor Tokio
               worker count may become an Actor population ceiling. */
    }
    requirement <'MAR-VM-R08'> reuseMailboxWaiting {
        doc /* Use the runtime mailbox's queue check, wait and wake protocol.
               Do not add a Scheme FIFO, idle polling loop or per-message wake
               service. Track allocation for native FIFO nodes, payloads,
               replies and typed boundary projections. Ring CPU amortization
               is mechanism evidence, not open-arrival business p99. */
    }
    requirement <'MAR-VM-R09'> moduleRetirement {
        doc /* Close admission before queued retirement, drain accepted messages,
               join only owned Actors and timers, reject outstanding timer slots
               and publish a checked receipt. A cancelled client does not abandon
               native resource custody. Other Modules continue independently. */
    }
    requirement <'MAR-VM-R10'> autonomousTimerWake {
        doc /* A timer must become observable while the host submits no further
               work. Native wait/wake owns progress, never a periodic Rust poll.
               Dropping a timer future abandons its observation, not its native
               operation; Module retirement remains responsible for joining it. */
    }
    requirement <'MAR-VM-R11'> qualifiedCostAccounting {
        doc /* Correlate GC and request age within the same owner turn. Generated
               constructor counters require exact source sites, zero counter
               errors, matching result bytes and one VM/processor admission.
               Instrumented times do not qualify production latency. A signed
               process-statistics delta is not physical allocation bytes. */
    }
    requirement <'MAR-VM-R12'> preservePublicationAuthority {
        doc /* Any compiler-state lifetime optimization belongs to the existing
               Gerbil/Yuanxu owners and must preserve native identity, fresh
               source guards, Feature selection and advanced Scheme extension.
               Actor shutdown cannot clear the compiler registry or substitute
               a remembered admission verdict. Qualify the complete Context
               workload separately from forced-GC retention diagnostics. */
    }
    allocation publicationStateAllocation allocate preservePublicationAuthority to singleVmRuntime.publicationState;
    allocation moduleRetirementAllocation allocate moduleRetirement to singleVmRuntime.moduleCatalog;
    allocation timerWakeAllocation allocate autonomousTimerWake to singleVmRuntime.moduleTimers;
    allocation costAccountingAllocation allocate qualifiedCostAccounting to singleVmRuntime.contextCosts;
    allocation populationAllocation allocate independentPopulationAndCredit to singleVmRuntime.actorPopulation;
    allocation readinessAllocation allocate independentPopulationAndCredit to singleVmRuntime.schedulerReadiness;
    allocation creditAllocation allocate independentPopulationAndCredit to singleVmRuntime.messageCredits;
    allocation waitingAllocation allocate reuseMailboxWaiting to singleVmRuntime.contextActors;
    dependency runtimeOwnsScheduling from MailboxScheduling to singleVmRuntime::schedulerReadiness;
    allocation compilationAllocation allocate linkedMailboxCompilation to singleVmRuntime.linkedRuntimeRoles;
    allocation ownerAllocation allocate singleVmOwnership to singleVmRuntime.owner;
    allocation batchAllocation allocate boundedReadyBatching to singleVmRuntime.owner;
    allocation retirementAllocation allocate terminalAccounting to singleVmRuntime.retirement;
    allocation measurementAllocation allocate measuredExecution to singleVmRuntime;
    allocation latencyAllocation allocate messageLatency to singleVmRuntime.contextActors;
    dependency highPerformanceRequirement from boundedReadyBatching to requirements::highPerformanceExecution;
}
