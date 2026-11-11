# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
#
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @just --list

# Validate the project entry and maintained OpenSysML models together.
sysml-check:
  sysml -strict -validate -self-check -no-record-cache project.sysml dev/sysml

# Run the package-scoped ASP Rust development gate across the workspace.
rust-dev-gate:
  cargo run --locked -p marlin-rust-project-harness-policy --example marlin-workspace-package-gate

# Install the dependency declared by the workspace Scheme package.
scheme-deps:
  cd scheme && gxpkg deps --install

# Run the package's build and test through the embedded Gerbil VM.
scheme-package-gate:
  direnv exec . bash -ec 'cd scheme && gxpkg env cargo test --locked -p marlin-gerbil-scheme --test unit_test scheme_package::gerbil_builds_and_tests_the_marlin_package_in_process -- --ignored --exact --nocapture'

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
