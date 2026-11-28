// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod agent_policy_routing;
mod c_abi;
mod dynamic;
mod dynamic_graph_policy;
mod graph_loop_continuation;
mod performance;
mod policy_pack_projection;
mod serde_payload;
pub(super) mod support;
mod typed_contract;

use support::assert_rust_projection_decode_error;
