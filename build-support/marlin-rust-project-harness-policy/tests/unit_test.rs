// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "unit/config/mod.rs"]
mod config;
#[path = "unit/dependency_topology.rs"]
mod dependency_topology;
#[path = "unit/evidence/mod.rs"]
mod evidence;
#[path = "unit/gerbil_runtime_assets.rs"]
mod gerbil_runtime_assets;
#[path = "support/mod.rs"]
pub mod policy;
#[path = "unit/rule_boundary.rs"]
mod rule_boundary;
#[path = "unit/scenario_benchmark.rs"]
mod scenario_benchmark;
#[path = "unit/workspace.rs"]
mod workspace;

asp_rust::asp_rust_cargo_test_gate!(mode = warn);
