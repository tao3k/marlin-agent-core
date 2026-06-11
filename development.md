# Development

## Format, test, lint

```shell
direnv exec . cargo fmt --all -- --check
direnv exec . cargo test
direnv exec . cargo clippy --all-targets --all-features -- -D warnings
direnv exec . git diff --check
```

Rust doc comments follow Clippy's `doc_markdown` style. Wrap API names,
literal identifiers, command names, and rule IDs in backticks rather than
leaving mixed-case technical terms as prose.

## Release visibility artifacts

The `real-gxi` workflow is a manual release boundary. Run it with
`workflow_dispatch` and `run_real_gxi=true` when a runner has Gerbil available
through `MARLIN_GERBIL_GXI` or Homebrew `gerbil-scheme`.

That job sets `MARLIN_RELEASE_STATUS_ARTIFACT_DIR` before running the ignored
`command::real_gxi` suite. The
`command_compiler_real_gxi_release_topology_persists_landing_status_sidecar`
test writes two downloadable artifacts into that directory:

- `release-status.json`: the persisted `.marlin/release-status.json` sidecar;
- `release-landing-report.json`: the compact landing report derived from the
  sidecar.

Use those artifacts as the CI evidence that the Gerbil release topology, local
`gxi` gate receipt, and release visibility report all crossed the real runtime
boundary.

## Crate naming boundaries

`marlin-agent-core` is a lean facade crate for stable imports and release
bridges. It should not become the implementation home for new runtime,
environment, hook, kernel, protocol, or harness behavior.

Place new behavior in the focused crates that own the boundary:

- `marlin-agent-protocol`: shared protocol data and trace/event contracts;
- `marlin-agent-runtime`: runtime traits, context, streams, and observability;
- `marlin-agent-kernel`: graph-loop execution and node adapters;
- `marlin-agent-hooks`: hook registration and dispatch;
- `marlin-agent-environment`: custom home, config layers, and sub-agent
  environment resolution;
- `marlin-agent-harness`: agent-system scenario, evidence, release visibility,
  and performance harnessing.

In this workspace, "harness" means the agent-system verification and evidence
surface. Do not use `marlin-agent-harness` APIs as a synonym for Rust's test
harness or `cargo test` mechanics.

## Self-applied policy

The crate runs its own project policy gate in both supported embedding modes:

- `src/lib.rs` mounts `src/self_policy.rs`, which installs
  `rust_project_harness_cargo_test_gate!`;
- `tests/unit_test.rs` stays a thin aggregate that only mounts suite modules.

When adding tests, keep behavior coverage under `tests/unit` and include it from
`tests/unit_test.rs` unless a new root target is intentionally part of the
policy surface.
Root Cargo test targets should stay as thin aggregates with external module
mounts only. Put test functions and helpers in suite files under `tests/unit`
or another documented suite directory.
Root-target module mounts should always be explicit `#[path = "suite/file.rs"]`
attributes rather than bare Rust `mod helper;` declarations.
Root `build.rs`, when present, is scanned by the project harness and should stay
a thin Cargo build-script entrypoint.

Intentional non-standard test roots or directories belong in
`tests/rust-project-harness-rules.toml`, and each entry must carry a non-empty
`explanation`.

## Policy closure

Default project assertions block on `Warning` and `Error`. `AGENT-*` rules stay
`Info`: rendered by default as repair advice, but non-blocking unless a caller
opts into stricter severity selection.

The `rust-project-harness` CLI follows the same contract. Compact text is the
default output for agent repair loops; `--json` emits the structured report for
tooling. CLI tests live under `tests/unit` and are mounted by the existing
`tests/unit_test.rs` target.

## Renderer snapshots

Compact text and JSON renderer contracts are locked by repo-local snapshot files
under `tests/unit/snapshots`. Update them only when the LLM-facing text shape or
structured JSON contract intentionally changes.
