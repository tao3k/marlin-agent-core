// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod abi_projection;
mod dynamic_graph_policy;
mod dynamic_runtime_bridge;
mod failures;
mod fixtures;
mod happy_path;
mod loop_governor_bridge;
mod policy_pack_projection;

use fixtures::{
    downstream_strategy_package, duplicate_abi_symbol_contract, graph_policy_abi_contract,
    graph_policy_abi_readiness_plan,
};
