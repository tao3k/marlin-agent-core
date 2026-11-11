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
                   permits other green Actors to run. Autonomous timers
                   between host requests still require a host wake bridge.
                   Module-owned retirement remains further integration work. */
        }
        part <'MAR-VM-A08'> linkedRuntimeRoles {
            doc /* The rooted C ABI imports the selected Deck and Context
                   runtime roles into the compiler-owned linked graph.
                   Yuanxu still owns checked publication and Feature selection.
                   Its role loader reuses the linked builtin identities.
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
               layout in the SDK compiler manifest. A separately compiled
               benchmark cannot qualify a dynamically loaded production role. */
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
