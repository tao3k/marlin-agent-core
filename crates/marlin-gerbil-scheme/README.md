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

The crate also ships a runnable source compiler and examples:

```sh
direnv exec . cargo run -p marlin-gerbil-scheme --example workspace-patch-intent
direnv exec . cargo run -p marlin-gerbil-scheme --example workspace-patch-intent-workflow
direnv exec . cargo run --locked -p marlin-gerbil-scheme --bin marlin-gerbil-compile-source -- \
  workspace-patch-intent crates/marlin-gerbil-scheme/examples/workspace-patch-intent-source.ss
```

Local verification without requiring Gerbil in default CI:

```sh
direnv exec . cargo test -p marlin-gerbil-scheme
direnv exec . cargo clippy -p marlin-gerbil-scheme --all-targets --all-features -- -D warnings
```

Performance benchmarks use Criterion and do not require a local Gerbil toolchain
for the Rust-side native ABI wrapper baseline:

```sh
direnv exec . cargo bench -p marlin-gerbil-scheme --bench deck_runtime_native -- --quick
```

To cross the real Scheme package boundary, run the same benchmark with an
explicit opt-in. This writes the crate-shipped `gerbil.pkg`, `build.ss`, and
`src/marlin/*` package assets, runs `build.ss compile`, and measures the
`gxi` process roundtrip through the Deck runtime policy selector:

```sh
MARLIN_GERBIL_REAL_PACKAGE_BENCH=1 \
  direnv exec . cargo bench -p marlin-gerbil-scheme --bench deck_runtime_native -- --quick
```

Use `MARLIN_GERBIL_GXI` to override the `gxi` executable path when the default
Homebrew path is not correct.

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

## Gerbil dependency bootstrap

`marlin-gerbil-deps` is the build-system entrypoint for preparing the external
Gerbil packages required by the deck runtime:

```sh
direnv exec . cargo run -p marlin-gerbil-scheme --bin marlin-gerbil-deps -- bootstrap
```

The command resolves its plan from the host platform and explicit
configuration:

```sh
direnv exec . cargo run -p marlin-gerbil-scheme --bin marlin-gerbil-deps -- env --print-plan
```

On macOS it can repair the Homebrew Gerbil/Gambit layout before building
`gerbil-utils` and `clan/poo`. On Linux and other platforms it skips the
Homebrew repair path and uses the configured or discovered `gxi`, `gxpkg`, and
`gsc` toolchain directly.

Cargo `build.rs` remains side-effect bounded: it runs the Rust harness policy
gate and does not fetch network dependencies or mutate `$HOME`. CI jobs,
packaging scripts, and local developer wrappers should call `marlin-gerbil-deps`
explicitly before running the real `gxi` boundary tests.

## Native AOT downstream integration boundary

This crate owns the Gerbil package, native AOT link-unit build helpers, and
Rust-side ABI types for the Deck runtime. It does not link a prebuilt
Gerbil/Gambit unit from its own `build.rs`, because that would push native
LLM/model-route integration costs into every consumer of the Scheme package.

Use `GerbilDeckRuntimeNativeAotConfig::build_link_unit` or the
`marlin-gerbil-native-aot` helper to produce the concrete module object, Gambit
link source, link object, and link metadata. The crate that performs real
LLM/model-route integration should decide whether and how to link those objects,
emit Cargo linker directives, and benchmark the linked path.

The local Criterion benchmark keeps the Rust ABI baseline, real Gerbil package
process boundary, and gated native AOT link-unit build timing here. Fully linked
runtime benchmarking still belongs next to the real LLM integration crate so
this package does not affect unrelated build and test chains.
