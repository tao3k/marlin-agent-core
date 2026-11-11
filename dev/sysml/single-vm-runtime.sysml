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
            doc /* Gerbil spawns a green Actor for each ready Context row.
                   Each Actor forces the lazy Lab Scope identity. The Scheme
                   owner joins all started Actors before publishing the ordered
                   result. The Rust host supplies transfer data and limits. */
        }
        part <'MAR-VM-A05'> retirement {
            doc /* Lab Context Scope digests cross as a packed u8vector.
                   Rust validates exactly 32 bytes per input row and copies the
                   full result once through the SDK bytevector projection.
                   Scope identity does not grant execution authority.
                   Cancellation and deadline are checked again before delivery. */
        }
        part <'MAR-VM-A06'> shutdown {
            doc /* Close admission, drain accepted requests, retire Context service,
                   release roots and drop the VM on its owner. Publish the typed
                   shutdown receipt after VM retirement and join the OS owner. */
        }
        part <'MAR-VM-A07'> applicationActors {
            doc /* Design for Module-owned Actors. Gerbil spawns and retires
                   Actors according to Module lifecycle and schedules their
                   mailboxes inside the VM. Tokio schedules Rust futures on
                   the host runtime. Actor population is independent of
                   Tokio worker count and Context C ABI transfer size.
                   Timers and mailboxes require a Scheme wait and wake path
                   between host entries. Population and lifecycle
                   qualification remain open. */
        }
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
               and Context rows per C ABI entry are bounded by batch capacity,
               at most 64, with at most 32 KiB of projected text. Refill the
               owner window between bounded VM turns. */
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
    allocation ownerAllocation allocate singleVmOwnership to singleVmRuntime.owner;
    allocation batchAllocation allocate boundedReadyBatching to singleVmRuntime.owner;
    allocation retirementAllocation allocate terminalAccounting to singleVmRuntime.retirement;
    allocation measurementAllocation allocate measuredExecution to singleVmRuntime;
    dependency highPerformanceRequirement from boundedReadyBatching to requirements::highPerformanceExecution;
}
