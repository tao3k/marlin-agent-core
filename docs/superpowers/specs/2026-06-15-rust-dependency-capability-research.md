# Rust Dependency Capability Research

Date: 2026-06-15
Status: research baseline for dependency capability planning

## Position

Marlin should not grow its Rust dependency set by collecting popular crates. Dependencies are capability investments. A crate is worth adding or promoting only when it closes a concrete agent-core reliability, performance, observability, protocol, storage, or testing gap that cannot be solved as well with the current workspace surface.

This document is therefore a capability-completion study, not a Cargo.toml shopping list.

The selection rule is:

1. Identify the root engineering problem.
2. Decide which crate boundary owns the capability.
3. Compare mature incumbents and newer crates that fix known ecosystem pain.
4. Require a verification gate before the dependency becomes part of the default workspace surface.

Age alone is not enough. Some old crates are stable but carry design debt, weak async support, hard-to-test behavior, or poor diagnostics. Some newer crates are better choices when they solve older ecosystem problems through clearer types, better concurrency, lower operational overhead, or stronger CI ergonomics.

## Current Baseline

The root workspace already pins several important foundations:

- Runtime and async surface: `tokio`, `tokio-stream`, `tokio-util`, `async-trait`, `parking_lot`.
- Serialization and configuration: `serde`, `serde_json`, `toml`, `orgize`.
- Stream and gateway surface: `litellm-rs` with `lite`, `gateway`, `sqlite`, `redis`, and `websockets` features enabled.
- Test and evidence foundation: `criterion`, `tempfile`, `rust-lang-project-harness`, `marlin-rust-project-harness-policy`.
- Graph/file matching foundation: `globset`.
- Error foundation: `thiserror`.

This is a good base, but it is not yet a complete agent-core capability platform. The main gaps are middleware resilience, structured subscriber setup, distributed observability, no-live-LLM test servers, schema-driven contract validation, concurrency model tests, stronger config diagnostics, file watching, credential storage, and memory/session persistence experiments.

## Reference Baseline

`.data/codex/codex-rs` exposes a useful reference shape:

- `schemars` and `ts-rs` for typed protocol and tool-schema surfaces.
- `opentelemetry` and `tracing-opentelemetry` in a distinct observability layer.
- `wiremock`, `insta`, `pretty_assertions`, `similar`, and `tempfile` as routine test infrastructure.
- `eventsource-stream`, `tokio-tungstenite`, `reqwest`, and `tokio-util` for streaming provider and app-server paths.
- `notify`, `ignore`, `walkdir`, and `globset` for file search and watcher functionality.
- `sqlx` for persisted state.
- `keyring` and secrets-oriented crates for credential boundaries.

`.data/pi` is TypeScript, so it should not drive Rust crate selection directly. It is still useful as a capability reference: provider SDK abstraction, SSE parsing tests, WebSocket probes, partial JSON cleanup, TypeBox schema discipline, ignore/glob workspace traversal, process execution, and lock-file handling.

The lesson from both references is not to copy their architecture. The lesson is to make every operational boundary testable without a live LLM and visible as typed evidence.

## Evaluation Rubric

Every candidate dependency should answer these questions before adoption:

- Capability: What root problem does it solve for agent core?
- Boundary: Which crate owns it, and which crates must not know about it?
- Hot path: Does it sit in request execution, loop scheduling, native ABI conversion, test-only code, or offline indexing?
- Performance: What overhead does it add under load, and what benchmark will catch regressions?
- Modernity: Does it fix known limitations in older libraries or patterns?
- Maturity: Is the API stable enough, and are maintenance/MSRV risks acceptable?
- Failure mode: What happens on cancellation, shutdown, corrupt input, network partition, file watcher overflow, or cache pressure?
- Feature control: Can it be feature-gated so downstream users do not pay for unused capabilities?
- Evidence: Which unit, integration, property, snapshot, or performance gate proves the dependency earns its place?

## P0: Promote Existing Foundations Into Capabilities

### Runtime Resilience: tower and tower-http

Root problem: provider calls, gateway routes, hook execution, and loop node execution need consistent timeout, concurrency, load-shed, retry-budget, and trace behavior.

Why it matters: without a middleware abstraction, each runtime edge grows its own timeout and retry policy, making evidence hard to compare.

Candidate role:

- `marlin-agent-runtime` owns service composition for loop/runtime edges.
- `marlin-agent-stream` owns gateway/provider stream service composition.
- `marlin-agent-protocol` must not depend on `tower`.

Modernity and performance: `tower::Service` gives a small, composable abstraction instead of framework-specific middleware. It is suitable when we own the Rust service boundary. It should not force a framework change if `litellm-rs` already owns a gateway surface.

Verification gate:

- Unit tests for timeout, cancellation, concurrency-limit, and load-shed receipts.
- No-live-provider tests using mock services.
- Benchmarks measuring per-request middleware overhead.

Reference: `ServiceBuilder` composes layers declaratively in Tower docs.

### Structured Observability: tracing-subscriber

Root problem: the workspace uses `tracing`, but it lacks a uniform subscriber policy for runtime spans, harness-local spans, JSON logs, and filter configuration.

Candidate role:

- `marlin-agent-runtime` owns process/runtime subscriber setup.
- `marlin-agent-harness` can keep harness-local recorders without installing a global subscriber.
- Test support owns deterministic captured span fixtures.

Verification gate:

- Snapshot tests for span/event fields.
- Tests proving harness-local traces do not leak global subscriber state.
- Redaction tests for provider tokens and native ABI payload summaries.

Reference: `tracing-subscriber` provides composable subscriber layers, and `EnvFilter` supports directive-based filtering.

### Gateway Streaming: reqwest, tokio-util, tokio-stream, bytes

Root problem: SSE, WebSocket, chunk gates, replay fixtures, and partial stream failures must be first-class runtime facts.

Current state: `litellm-rs` is already enabled with gateway, sqlite, redis, and websockets. The gap is not feature activation; the gap is reusable stream test and evidence infrastructure.

Candidate role:

- `marlin-agent-stream` owns provider and gateway streaming.
- `marlin-agent-test-support` owns fake SSE/WebSocket servers, chunk gates, and malformed-stream fixtures.
- `marlin-agent-harness` consumes typed stream receipts, not raw logs.

Verification gate:

- SSE parsing fixtures.
- WebSocket disconnect/reconnect fixtures.
- Chunk boundary and unicode surrogate tests.
- No-live-LLM denial receipts.

## P1: Add Capability Libraries With Strong Payoff

### Test Evidence: wiremock, insta, proptest, loom, and shuttle

Root problem: agent systems need strong tests without real LLM calls. Simple unit tests do not cover provider contracts, stream ordering, graph loop invariants, or concurrency bugs.

Recommended split:

- `wiremock` for deterministic HTTP/provider mock servers.
- `insta` for large protocol, receipt, contract, and trace snapshot review.
- `proptest` for type invariants, native ABI projections, graph-loop policy invariants, and Org contract generation.
- `loom` for small concurrency primitives where exhaustive interleaving is tractable.
- `shuttle` as a newer randomized concurrency-testing candidate for larger scenarios where Loom's exhaustive model becomes too expensive.

Modernity and performance: `shuttle` is not a replacement for Loom. It trades proof strength for scale. That is valuable for runtime scheduler and session isolation tests that are too large for exhaustive permutation.

Verification gate:

- All no-live-LLM tests must fail closed if a real provider boundary is touched.
- Snapshot tests must redact tokens and unstable paths.
- Property tests must persist counterexamples.
- Concurrency tests must be explicitly scoped and not run in every fast local check unless configured.

### Contract and Config Diagnostics: schemars, serde_path_to_error, miette

Root problem: typed TOML, Org contracts, tool schemas, provider configs, and memory/session envelopes need high-quality diagnostics. Bad config should point to the exact field, not produce a generic deserialize error.

Candidate role:

- `schemars` derives external JSON Schema for protocol/config surfaces that need machine-readable validation.
- `serde_path_to_error` enriches deserialization errors with field paths.
- `miette` provides user-facing diagnostic reports with codes, labels, and help text.

Boundary rule: this does not change the native Gerbil ABI rule. Scheme-native integration remains Scheme types -> Rust types. JSON Schema is for external configuration, protocol documentation, test fixtures, and contract validation, not for native ABI transport.

Verification gate:

- Snapshot diagnostics for malformed TOML and contract files.
- Schema export tests for stable public contract surfaces.
- Negative tests for ambiguous legacy config keys.

### Observability Export: opentelemetry and tracing-opentelemetry

Root problem: graph-loop execution, sessions, hooks, gateway calls, and test harness runs need a correlatable trace model when debugging real workloads.

Candidate role:

- A future `marlin-agent-observability` or runtime-owned feature-gated module owns export setup.
- Core protocol crates should expose typed trace identifiers and receipts, not depend on OTel exporters.

Modernity and performance: OTel should be optional and feature-gated. Runtime receipts must remain useful without an OTel collector. Export failures must not break loop execution.

Verification gate:

- Unit tests for span-to-receipt correlation IDs.
- Feature-gated smoke test that exports to a test collector or in-memory exporter.
- Benchmarks proving disabled OTel has negligible overhead.

## P2: Storage, Memory, and Indexing Experiments

### Persistent State: sqlx and redb

Root problem: sessions, rollout traces, memory indexes, and replay receipts need persistence with crash behavior that can be tested.

Recommended posture:

- Use `sqlx` when relational queries, migrations, and offline checked SQL matter.
- Evaluate `redb` when an embedded, pure-Rust, zero-copy, ACID key-value store fits local session or evidence storage better than SQLite.

Modernity and risk: `redb` is attractive because it is pure Rust, ACID, MVCC-capable, and avoids RocksDB-style operational weight. It still needs crash/recovery, file-size, compaction, and concurrent-reader testing before becoming a default.

Verification gate:

- Migration tests for `sqlx`.
- Crash/reopen tests for `redb`.
- Concurrent reader/writer tests.
- Replay determinism tests for persisted receipts.

### Search and Recall: tantivy

Root problem: Org memory, session recall, and evidence graph discovery need indexed text search and ranked retrieval without handing agents raw transcript blobs.

Candidate role:

- `marlin-org-memory` or a future memory-index crate owns search indexes.
- Runtime consumes compact context packs, not search engine internals.

Modernity and performance: Tantivy is a Rust Lucene-like search library. It is appropriate for local full-text indexing, but it should not become the only memory graph model. It indexes text; it does not replace typed graph edges.

Verification gate:

- Golden recall fixtures.
- Index rebuild and incremental update tests.
- Benchmarks for indexing latency and query latency.

### Cache Layer: moka, foyer, and scc

Root problem: context packs, parsed Org facts, schema projections, and provider metadata need bounded, concurrent caching.

Recommended posture:

- `moka` is the first in-memory async cache candidate because it is mature and futures-aware.
- `foyer` is a serious experimental candidate when hybrid memory/disk cache is needed.
- `scc` is a candidate for high-concurrency maps/caches where `DashMap` or lock-based maps become bottlenecks.

Modernity and risk: newer cache libraries may fix old LRU and lock-contention problems, but they need workload-specific benchmarks. Do not adopt a cache because it is faster in abstract; adopt it only after it improves a Marlin workload.

Verification gate:

- Cache pressure tests.
- Cancellation tests for async cache loading.
- Benchmarks with representative context-pack and parsed-Org workloads.

## P3: Workspace, Security, and Protocol Extensions

### Workspace File Events: notify, ignore, walkdir, camino, compact_str

Root problem: workspace evidence graph updates, file watchers, and memory indexing need correct path handling and efficient small-string behavior.

Candidate role:

- File watching belongs in workspace or memory-index crates.
- `camino` is useful when public APIs require UTF-8 paths.
- `compact_str` is an optimization candidate for repeated small identifiers, node ids, headings, labels, and source-span keys.

Verification gate:

- Cross-platform path tests.
- Watch overflow and rename tests.
- Allocation benchmarks before adopting compact string types in public structs.

### Secrets: keyring, secrecy, and zeroize

Root problem: provider tokens, gateway credentials, and local secret material must not leak into logs, snapshots, traces, or panic output.

Candidate role:

- A secrets boundary should own credential loading and storage.
- Protocol and receipt types should carry redacted summaries.
- Test support must assert redaction.

Verification gate:

- Snapshot tests proving redaction.
- Platform-gated keyring smoke tests.
- Panic/debug formatting tests.

## Explicit Non-goals

### MCP and rmcp

MCP is not part of the current Marlin design system and should not be treated as a dependency capability lane in this research cycle. `rmcp` must not be promoted into the current workspace plan, and no `marlin-agent-mcp` crate should be inferred from this document.

Codex contains MCP-oriented surfaces, but they are only reference evidence that other agent systems may choose this integration boundary. Marlin's current design work is focused on graph-loop runtime, typed protocol receipts, Org contracts, streaming, sessions, memory, hooks, native Gerbil policy, and evidence-driven test infrastructure.

If MCP becomes relevant later, it should start as a separate design document with its own ownership, threat model, dependency review, and no-live-network test plan. It should not enter through this dependency capability baseline.

## Dependency Introduction Policy

New dependency proposals should include:

- Cargo feature plan and default-feature decision.
- Owning crate.
- Non-owning crates that must not depend on it.
- Test-support plan.
- Performance or concurrency gate if it touches runtime hot paths.
- Failure-mode tests.
- CI impact estimate.
- Nix/build impact estimate.
- Removal plan if the experiment fails.

The first adoption step should usually be a dev-dependency or feature-gated experiment, not immediate promotion to a default workspace dependency.

## Recommended First Experiments

1. Add a `marlin-agent-test-support` no-live-LLM fixture layer using `wiremock`, `insta`, and property-test counterexample persistence.
2. Add structured tracing setup using `tracing-subscriber`, with deterministic harness-local span capture kept separate.
3. Prototype `tower` around one runtime/provider edge and measure timeout, cancellation, and concurrency-limit evidence.
4. Add config diagnostics with `serde_path_to_error` and `miette` for one TOML or Org contract boundary.
5. Evaluate `sqlx` versus `redb` for session/evidence persistence with crash/reopen tests.
6. Benchmark `moka` versus `foyer` for parsed Org/context-pack caching before choosing a cache layer.

## Source Notes

- Tower `ServiceBuilder`: https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html
- `tracing-subscriber` and `EnvFilter`: https://docs.rs/tracing-subscriber, https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html
- OpenTelemetry Rust and `tracing-opentelemetry`: https://opentelemetry.io/docs/languages/rust/, https://docs.rs/tracing-opentelemetry
- `schemars`: https://docs.rs/schemars
- `serde_path_to_error`: https://docs.rs/serde_path_to_error
- `miette`: https://docs.rs/miette/latest/miette/
- `insta`, `proptest`, `loom`, `shuttle`: https://docs.rs/insta, https://docs.rs/proptest, https://docs.rs/loom/latest/loom/, https://docs.rs/shuttle/latest/shuttle/
- `moka`, `foyer`, `scc`: https://docs.rs/moka/latest/moka/, https://docs.rs/foyer, https://docs.rs/scc
- `redb`, `tantivy`: https://docs.rs/redb, https://docs.rs/tantivy/
- `camino`, `compact_str`: https://docs.rs/camino, https://docs.rs/compact_str
- `iai-callgrind` and cargo-nextest: https://docs.rs/iai-callgrind, https://nexte.st/
- `keyring`: https://docs.rs/keyring
