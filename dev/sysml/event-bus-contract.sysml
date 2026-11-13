// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinEventBusContract {
    doc /* Proposed typed event-bus protocol. The production text bus now
           reserves every live subscriber before publication, including a
           cancellation path, but still lacks byte budgets and typed graph/tool
           payloads. This bounded model is not a persistence guarantee or a
           telemetry backend. */
    part <'MAR-BUS-A01'> eventBusDesign {
        doc /* Typed events carry identity and lifecycle state; resource owners
               remain in Runtime. The design prototype is dev/verification/event_bus_contract.rs. */
    }
    requirement <'MAR-BUS-R01'> atomicReliableAdmission {
        doc /* A reliable publish is admitted to every active subscriber or none.
               Item and byte budgets are reserved before admission; unavailable
               capacity returns backpressure without partial delivery. */
    }
    requirement <'MAR-BUS-R02'> releaseOnConsumeOrClose {
        doc /* A subscriber owns its reserved capacity until it consumes the event
               or closes. Closing releases that subscriber's reservations. */
    }
    requirement <'MAR-BUS-R03'> boundedModel {
        doc /* Quint dev/quint/event-bus.qnt checks finite two-subscriber capacity,
               atomic admission, release and a deliberate over-admission mutant. */
    }
    requirement <'MAR-BUS-R04'> graphGateWake {
        doc /* The runtime graph-loop registry owns pending human gates. Its
               revision watch is a coalescing wake hint, never the gate payload
               or authority. Subscribers inspect the typed gate by run ID after
               subscribing and after each change; a slow subscriber must not
               lose a gate because intermediate revisions were coalesced. */
    }
}
