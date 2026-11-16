// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_gerbil_scheme::{GerbilAotConfig, GerbilAotStaticLinkStatus};
use tempfile::Builder;

#[test]
fn graph_aot_static_link_plan_rejects_non_ready_object_receipt() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-link-not-ready-")
        .tempdir()
        .expect("create root");

    let receipt = GerbilAotConfig::agent_policy_routing(root.path())
        .with_gsc(root.path().join("missing-gsc"))
        .build_link_unit();
    let link_plan = receipt.static_link_plan();

    assert_eq!(
        link_plan.status,
        GerbilAotStaticLinkStatus::LinkUnitNotReady
    );
    assert!(link_plan.cargo_directives.is_empty());
    assert!(link_plan.detail.as_deref().is_some_and(|detail| {
        detail.contains("native AgentGraph policy routing link unit is not ready")
    }));
}
