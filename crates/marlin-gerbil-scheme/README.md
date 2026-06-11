# marlin-gerbil-scheme

`marlin-gerbil-scheme` provides the Rust binding surface for running Gerbil
Scheme sources as typed Marlin artifacts. The crate ships the Scheme adapter
modules needed by `gxi`, writes them into a `GERBIL_LOADPATH` root, and invokes
the `:marlin/adapter` module through a JSON stdin/stdout protocol.

The crate is currently distributed as part of the Marlin workspace. It is
marked `publish = false` until the workspace's internal crate dependency chain
has a separate crates.io release plan.

The default runtime binding uses `MARLIN_GERBIL_GXI` when it is set, otherwise
it falls back to the Homebrew `gerbil-scheme` executable path.

```rust
let compiler = marlin_gerbil_scheme::GerbilCommandCompiler::from_default_marlin_runtime_module(
    std::env::temp_dir().join("marlin-gerbil-runtime"),
)?;
```

For custom process control, build a `GerbilCommandSpec` or
`GerbilCommandProfile` directly and pass it to `GerbilCommandCompiler`.

The crate also ships runnable examples:

```sh
direnv exec . cargo run -p marlin-gerbil-scheme --example workspace-patch-intent
direnv exec . cargo run -p marlin-gerbil-scheme --example workspace-patch-intent-workflow
direnv exec . cargo run --locked -p marlin-gerbil-scheme --example compile-source -- \
  workspace-patch-intent crates/marlin-gerbil-scheme/examples/workspace-patch-intent-source.ss
```

Local verification without requiring Gerbil in default CI:

```sh
direnv exec . cargo test -p marlin-gerbil-scheme
direnv exec . cargo clippy -p marlin-gerbil-scheme --all-targets --all-features -- -D warnings
```

On machines with `gxi`, run the ignored runtime boundary suite:

```sh
artifact_dir="$(mktemp -d)"
MARLIN_REQUIRE_REAL_GXI=1 \
  MARLIN_RELEASE_STATUS_ARTIFACT_DIR="$artifact_dir" \
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored --nocapture
ls "$artifact_dir"/release-status.json "$artifact_dir"/release-landing-report.json
```

The same artifact contract is used by the manual `real-gxi` CI workflow so
release visibility evidence is available after the ignored suite crosses the
local Gerbil runtime boundary.
