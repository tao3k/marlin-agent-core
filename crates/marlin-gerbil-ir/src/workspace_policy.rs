// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Workspace policy `IR` emitted by the `Gerbil` control plane.

use serde::{Deserialize, Serialize};

/// Release topology for a crate or artifact family managed by `Gerbil`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseTopologySpec {
    pub topology_id: String,
    pub crate_name: String,
    pub publish_enabled: bool,
    pub asset_audit_command: String,
    pub package_assets: Vec<String>,
    pub runtime_dependency_chain: Vec<String>,
    pub workflow_dependency_chain: Vec<String>,
    pub gates: Vec<ReleaseGateSpec>,
}

/// Visibility evidence expected from a release gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseVisibilitySpec {
    pub report_key: String,
    pub evidence_keys: Vec<String>,
    pub artifact_paths: Vec<String>,
}

/// Release gate command and artifacts required by a `Gerbil` release topology.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGateSpec {
    pub gate_id: String,
    pub command: String,
    pub requires_local_gerbil: bool,
    pub required_artifacts: Vec<String>,
    pub visibility: Vec<ReleaseVisibilitySpec>,
}
