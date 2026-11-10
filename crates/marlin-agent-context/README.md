# Marlin Agent Context

Owns Marlin's Context requests, batch admission and scope digest projections.
The crate builds without a Scheme SDK by default. The `gerbil` feature calls
the build-owned Context entry on its VM owner using gerbil-scheme-rust's
rooted-object ABI. The Scheme adapter lives in `scheme/context/` and uses
AI Agentic Lab's compiled Context library.

A batch shares its bundle, organization, project, worktree, epoch and source
cut. Each row carries its own actor, task, destination and optional
session/turn pair. The caller may dispatch one ready row immediately.
The ABI returns one fixed-width Lab digest per row in submission order.
These digests identify admitted scopes. Rust must still check that the scope
and grants are current before admitting effects.

Linking the Context export into the production VM image, routing live Agent
Calls, and qualifying throughput and tail latency under load are separate
integration steps.

Run `cargo test -p marlin-agent-context` without a VM and
`cargo bench -p marlin-agent-context --bench scope_batch` to measure Rust
request preparation for 1, 8, 32 and 64 rows. The benchmark does not include
Gerbil root construction, C ABI dispatch or Lab Scope validation.
