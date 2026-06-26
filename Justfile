set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

gerbil_deps := "direnv exec . cargo run -p marlin-gerbil-scheme --bin marlin-gerbil-deps --"

default:
  @just --list

# Print the Rust-resolved Gerbil dependency bootstrap plan.
gerbil-env:
  {{gerbil_deps}} env

# Repair platform-specific Gerbil package tooling state.
gerbil-homebrew-repair:
  {{gerbil_deps}} repair

# Fetch local Gerbil package dependency checkouts.
gerbil-deps-fetch:
  {{gerbil_deps}} fetch

# Link Gerbil packages into the user global gxpkg environment.
gerbil-deps-link:
  {{gerbil_deps}} link

# Build Gerbil package dependencies through the Rust bootstrapper.
gerbil-deps-build:
  {{gerbil_deps}} build

# Verify Gerbil can import the deck-runtime dependency modules.
gerbil-deps-verify:
  {{gerbil_deps}} verify

# Full local bootstrap for deck-runtime Gerbil dependencies.
gerbil-deck-runtime-bootstrap:
  {{gerbil_deps}} bootstrap

# Run the real-gxi gates that prove Rust can execute against installed Gerbil packages.
gerbil-real-gxi-gate: gerbil-deps-verify
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test \
    command::real_gxi::examples::poo_object::command_compiler_real_gxi_deck_runtime_can_execute_poo_object_probe_when_dependency_installed \
    -- --ignored --nocapture
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test \
    command::real_gxi::examples::runtime_assets::command_compiler_real_gxi_deck_runtime_capability_handshake \
    -- --ignored --nocapture
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test \
    command::real_gxi::examples::runtime_assets::command_compiler_real_gxi_deck_runtime_selects_scheme_model_route_policy \
    -- --ignored --nocapture
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test \
    deck_runtime_policy::gerbil_deck_runtime_policy_runtime_binding_real_gxi_selects_policy \
    -- --ignored --nocapture
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test \
    deck_runtime_native::gerbil_deck_runtime_native_selector_performance_gate_stays_in_process \
    -- --nocapture
  direnv exec . cargo test -p marlin-gerbil-scheme --test unit_test \
    command::real_gxi::examples::build_script::command_compiler_real_gxi_build_script_compiles_runtime_assets \
    -- --ignored --nocapture

# Smoke gate for Gerbil package compile, module pack tests, and downstream module examples.
gerbil-package-module-gate:
  cd crates/marlin-gerbil-scheme/gerbil && GERBIL_LOADPATH=src gxi build.ss compile
  cd crates/marlin-gerbil-scheme/gerbil && GERBIL_LOADPATH=src gxtest t/marlin-policy-pack-inventory-test.ss
  cd crates/marlin-gerbil-scheme/gerbil && GERBIL_LOADPATH=src gxtest t/marlin-user-interface-prefab-test.ss
  cd crates/marlin-gerbil-scheme/examples/user-interface-module-config && GERBIL_LOADPATH=../../gerbil/src:. gxtest t/user-interface-module-config-test.ss

# Smoke gate for real AgentGraph policy-routing AOT staging plus linked native Rust tests.
agent-policy-routing-native-gate:
  stage_dir="$(mktemp -d "${TMPDIR:-/tmp}/marlin-agent-policy-routing-aot.XXXXXX")"; \
    cd crates/marlin-gerbil-scheme/gerbil && \
    gxi build.ss stage-native-aot agent-policy-routing "$stage_dir"
  direnv exec . cargo test -p marlin-agent-policy-routing-native --features linked-native

# Benchmark build gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-native-bench-build:
  direnv exec . cargo bench -p marlin-agent-policy-routing-native --features linked-native --bench linked_selector --no-run

# Allocation-shape performance gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-native-performance-gate:
  direnv exec . cargo bench --locked -p marlin-agent-policy-routing-native --features linked-native --bench allocation_shape

# Full benchmark gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-native-bench:
  direnv exec . cargo bench -p marlin-agent-policy-routing-native --features linked-native --bench linked_selector

# Local P0-P4 gate for AgentGraph native policy-routing and Gerbil module-system closure.
agent-graph-native-routing-gate: gerbil-package-module-gate agent-policy-routing-native-gate agent-policy-routing-native-bench-build agent-policy-routing-native-performance-gate
