// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinTelemetry {
    part <'MAR-TEL-A01'> telemetry {
        doc /* crates/marlin-agent-telemetry owns local resource observations,
               tracing subscribers, the OpenTelemetry provider, the bounded
               batch queue and OTLP/HTTP trace export. */
    }
    part <'MAR-TEL-A02'> terminalEntry {
        doc /* The TUI entry point explicitly opts in and closes telemetry
               after its Runtime and UI have drained. */
    }
    requirement <'MAR-TEL-R01'> allowlistedSpans {
        doc /* Export spans from the selected instrumentation target. The
               terminal selects Runtime lifecycle span metadata. */
    }
    requirement <'MAR-TEL-R02'> drainOnShutdown {
        doc /* End the trace provider with a bounded shutdown call after
               application work ends. Test the emitted OTLP payload. */
    }
    requirement <'MAR-TEL-R03'> silentCollectorShutdown {
        doc /* On macOS the linked TUI must restore the terminal and drain
               Runtime ownership before attempting its final OTLP export.
               A collector that accepts the request and withholds a response
               must not keep the TUI alive past the PTY shutdown deadline. */
    }
    requirement <'MAR-TEL-R04'> localProcessSampling {
        doc /* On macOS sample host and managed direct-child physical footprint
               separately through the protocol port. Local sampling operates
               independently of the collector. Runtime applies its own
               admission policy and receives sampling failures as failures. */
    }
    allocation selectSpans allocate allowlistedSpans to telemetry;
    allocation finishExport allocate drainOnShutdown to telemetry;
    allocation invokeShutdown allocate drainOnShutdown to terminalEntry;
    allocation collectorIndependentDrain allocate silentCollectorShutdown to terminalEntry;
    allocation sampleLocally allocate localProcessSampling to telemetry;
    dependency observesRuntime from telemetry to MarlinProcessLifecycle::managedProcess;
}
