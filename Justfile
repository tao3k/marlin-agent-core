# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
#
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Bound native Scheme builds to the available processors unless the caller
# has selected a smaller budget.
export GERBIL_BUILD_CORES := env_var_or_default("GERBIL_BUILD_CORES", num_cpus())

default:
  @just --list

# Validate the project entry and maintained OpenSysML models together.
sysml-check:
  sysml -strict -validate -self-check -no-record-cache project.sysml dev/sysml

# Run the ASP Rust native workspace Dev Gate through Cargo test.
rust-dev-gate:
  cargo test --locked -p marlin-rust-project-harness-policy --test workspace_policy

# Authored Quint models and reproducible trace checks.
property-model-gate: app-server-model-gate
  quint typecheck dev/quint/host-observer.qnt
  quint run dev/quint/host-observer.qnt --seed=42 --max-samples=100 --max-steps=8 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/host-observer.qnt --step=badStep --invariant=safe --seed=42 --max-samples=100 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in host exit negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/runtime-process-owner.qnt
  quint test dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner
  quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --seed=42 --max-samples=1000 --max-steps=8 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --step=watchdogMutation --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in watchdog negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @output=$(quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --step=healthMutation --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in health negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @output=$(quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process owner negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/event-bus.qnt
  quint test dev/quint/event-bus.qnt --main=eventBus
  quint run dev/quint/event-bus.qnt --main=eventBus --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/event-bus.qnt --main=eventBus --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=10 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in event bus negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @test_binary=$(mktemp -t marlin-event-bus-test.XXXXXX); trap 'rm -f "$test_binary"' EXIT; rustc --edition=2024 --test dev/verification/event_bus_contract.rs -o "$test_binary"; "$test_binary"
  quint typecheck dev/quint/tui-minibuffer.qnt
  quint test dev/quint/tui-minibuffer.qnt --max-samples=100
  quint run dev/quint/tui-minibuffer.qnt --seed=42 --max-samples=1000 --max-steps=30 --invariant=stateSafe --verbosity=1
  quint typecheck dev/quint/model-tool-review.qnt
  quint test dev/quint/model-tool-review.qnt --main=modelToolReview --max-samples=100
  quint run dev/quint/model-tool-review.qnt --main=modelToolReview --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/model-tool-review.qnt --main=modelToolReview --step=badStep --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in model tool review negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/strict-tree-proposal.qnt
  quint test dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal
  quint run dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal --seed=42 --max-samples=1000 --max-steps=12 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal --step=badLaunchStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in strict launch negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @output=$(quint run dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal --step=badReleaseStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in strict cleanup negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/process-lease.qnt
  quint test dev/quint/process-lease.qnt --main=processLease
  quint run dev/quint/process-lease.qnt --main=processLease --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/process-lease.qnt --main=processLease --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=10 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process lease negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/process-tree.qnt
  quint test dev/quint/process-tree.qnt --main=processTree
  quint run dev/quint/process-tree.qnt --main=processTree --seed=42 --max-samples=1000 --max-steps=8 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/process-tree.qnt --main=processTree --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process tree negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/process-quota.qnt
  quint test dev/quint/process-quota.qnt --main=processQuota --max-samples=100
  quint run dev/quint/process-quota.qnt --main=processQuota --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1
  @output=$(quint run dev/quint/process-quota.qnt --main=processQuota --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process quota negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }

# Run only inside a disposable, root-owned delegated Linux systemd scope.
strict-linux-os-qualification:
  python3 dev/verification/linux_cgroup_qualification.py

# Session journal, reconnect cursor and Runtime cancellation.
app-server-model-gate:
  quint typecheck dev/quint/app-server-turn.qnt
  quint test dev/quint/app-server-turn.qnt --max-samples=100
  quint run dev/quint/app-server-turn.qnt --seed=42 --max-samples=1000 --max-steps=30 --invariant=safe --verbosity=1

# One project gate: architecture, model, native Scheme PBT and Rust properties.
property-gate: sysml-check property-model-gate scheme-property-gate
  cargo test --locked -p marlin-agent-telemetry
  cargo test --locked -p marlin-agent-runtime --test unit_test process::quota
  cargo test --locked -p marlin-agent-runtime --test unit_test process::containment
  cargo test --locked -p marlin-agent-runtime --test unit_test process::managed_executor
  cargo test --locked -p marlin-agent-runtime --test unit_test runtime_task_tracker
  cargo test --locked -p marlin-git-utils
  cargo test --locked -p marlin-agent-runtime --lib confirmed_process_does_not_release_until_pipe_endpoint_is_dropped
  cargo test --locked -p marlin-agent-kernel --test unit_test strict_tree_
  python3 dev/verification/process_quota_gate.py
  cargo test --locked -p marlin-agent-tui --no-default-features --lib
  cargo test --locked -p marlin-agent-ui --test ui_native_projection
  if [[ $(uname -s) == Darwin ]]; then just tui-runtime-gate; fi

# Real linked macOS TUI: override MARLIN_TUI_SOAK_SECONDS for longer runs.
tui-runtime-gate:
  just tui-aot-stage
  MARLIN_TUI_SOAK_SECONDS="${MARLIN_TUI_SOAK_SECONDS:-30}" cargo test --locked -p marlin-agent-tui --test pty repeated_stream_cancellation_soak -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty workspace_tool_cancel_reaps_process -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty workspace_tool_shutdown_reaps_after_exit -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty stalled_collector_does_not_delay_runtime_drain_or_block_tui_exit -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty quiet_workspace_tool_survives_idle_observation -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty process_buffer_stops_a_managed_generation -- --ignored --nocapture
  @trace_dir=$(mktemp -d -t marlin-host-trace.XXXXXX); trap 'rm -rf "$trace_dir"' EXIT; quint test dev/quint/host-observer.qnt --out-itf="$trace_dir/{test}-{seq}.json"; trace_files=("$trace_dir"/*.json); [[ ${#trace_files[@]} == 1 && -f "${trace_files[0]}" ]]; MARLIN_HOST_OBSERVER_TRACE="${trace_files[0]}" cargo test --locked -p marlin-agent-tui --test pty independent_observer_distinguishes_stall_from_exit -- --ignored --nocapture

# Gerbil's native test runner discovers the selected package's maintained suite.
scheme-property-gate:
  python3 -m unittest discover -s dev/verification -p test_gerbil_gate.py
  python3 dev/verification/run_gerbil_property_gate.py
  python3 dev/verification/run_gerbil_property_gate.py t/ui-native-test.ss

# Install the dependency declared by the workspace Scheme package.
scheme-deps:
  cd scheme && gerbil deps --install

# Run the package's build and test through the embedded Gerbil VM.
scheme-package-gate:
  direnv exec . bash -ec 'cd scheme && gxpkg env cargo test --locked -p marlin-gerbil-scheme --test unit_test scheme_package::gerbil_builds_and_tests_the_marlin_package_in_process -- --ignored --exact --nocapture'

# Build the init-selected Scheme graph through the package Builder.
scheme-build:
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ "$(uname -s)" == Darwin ]]; then
    unset SDKROOT DEVELOPER_DIR NIX_CFLAGS_COMPILE
    export PATH="${GERBIL_PREFIX:+$GERBIL_PREFIX/bin:}/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
  fi
  cd scheme
  gerbil build

# Stage the selected package's linked TUI programme.
tui-aot-stage: scheme-build tui-program-stage

# Consume the Scheme package already built by scheme-build.
tui-program-stage:
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ "$(uname -s)" == Darwin ]]; then
    unset SDKROOT DEVELOPER_DIR NIX_CFLAGS_COMPILE
    export PATH="${GERBIL_PREFIX:+$GERBIL_PREFIX/bin:}/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
  fi
  cd scheme
  gxpkg env gxc -S -s -d .gerbil/lib tui-entry.ss
  gxpkg env gxi tui-stage.ss

# Build the default linked debug binary used by target/debug/marlin-tui.
tui-aot-debug: tui-aot-stage
  cargo build --locked -p marlin-agent-tui --bin marlin-tui

# Link the staged Scheme graph and VM into the terminal application.
tui-aot-build: tui-aot-stage
  cargo build --locked -p marlin-agent-tui --bin marlin-tui --features linked-ui --release

# Keep interaction latency, first-copy startup, and source-free release
# qualification in one reproducible command. Build time is not a sample.
tui-performance-bench: tui-aot-debug tui-aot-build
  cargo test --locked -p marlin-agent-tui --test launch linked_debug_pre_ui_launch_pair -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test launch linked_release_pre_ui_launch_pair -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty first_mx_arrow_is_not_lost_before_the_menu_frame
  cargo test --locked -p marlin-agent-tui --test pty mx_navigation_pty_latency -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty debug_linked_fresh_copy_startup_pair -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty release_linked_fresh_copy_startup_pair -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty release_linked_ui_first_frame_without_a_gerbil_package_tree -- --ignored --nocapture

# Diagnose whether the size-oriented profile improves a source-matched linked
# TUI. Stage the Scheme programme first; comparing against a stale native
# manifest can produce a misleading projection failure.
tui-size-performance-bench: tui-aot-stage
  cargo build --locked -p marlin-agent-tui --bin marlin-tui --features linked-ui --profile release-size
  cargo test --locked -p marlin-agent-tui --test launch linked_release_size_pre_ui_launch_pair -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty release_size_linked_fresh_copy_startup_pair -- --ignored --nocapture

# Exercise the linked AgentGraph policy selector through its typed C ABI.
agent-policy-routing-abi-gate:
  direnv exec . cargo test --locked -p marlin-agent-policy-routing-abi --features linked-selector

# Benchmark build gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-abi-bench-build:
  direnv exec . cargo bench -p marlin-agent-policy-routing-abi --features linked-selector --bench linked_selector --no-run

# Allocation-shape performance gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-abi-performance-gate:
  direnv exec . cargo bench --locked -p marlin-agent-policy-routing-abi --features linked-selector --bench allocation_shape

# Full benchmark gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-abi-bench:
  direnv exec . cargo bench -p marlin-agent-policy-routing-abi --features linked-selector --bench linked_selector

# Local P0-P4 gate for AgentGraph native policy-routing and Gerbil module-system closure.
agent-graph-abi-routing-gate: scheme-package-gate agent-policy-routing-abi-gate agent-policy-routing-abi-bench-build agent-policy-routing-abi-performance-gate
