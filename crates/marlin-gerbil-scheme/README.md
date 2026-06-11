# marlin-gerbil-scheme

`marlin-gerbil-scheme` provides the Rust binding surface for running Gerbil
Scheme sources as typed Marlin artifacts. The crate ships the Scheme adapter
modules needed by `gxi`, writes them into a `GERBIL_LOADPATH` root, and invokes
the `:marlin/adapter` module through a JSON stdin/stdout protocol.

The crate is currently distributed as part of the Marlin workspace. It is marked
`publish = false` until the workspace's internal crate dependency chain has a
separate crates.io release plan.

The default runtime binding uses `MARLIN_GERBIL_GXI` when it is set, otherwise
it falls back to the Homebrew `gerbil-scheme` executable path.

```rust
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCompiler, GerbilSource,
};

let runtime_root = std::env::temp_dir().join("marlin-gerbil-runtime");
let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(&runtime_root)?;
let artifact = compiler.compile(
    GerbilSource::new("workspace-patch-intent", "(workspace-patch-intent \"intent:memory\")"),
    GerbilArtifactKind::WorkspacePatchIntent,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For custom process control, build a `GerbilCommandSpec` or
`GerbilCommandProfile` directly and pass it to `GerbilCommandCompiler`.

The crate also ships runnable examples:

```sh
cargo run -p marlin-gerbil-scheme --example workspace-patch-intent
cargo run -p marlin-gerbil-scheme --example workspace-patch-intent-workflow
```

Local verification without requiring Gerbil in default CI:

```sh
cargo test -p marlin-gerbil-scheme
cargo check -p marlin-gerbil-scheme --examples
cargo package -p marlin-gerbil-scheme --list --allow-dirty
```

On machines with `gxi`, run the ignored runtime boundary suite:

```sh
cargo test -p marlin-gerbil-scheme command::real_gxi -- --ignored --nocapture
```
