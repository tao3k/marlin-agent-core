use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeStaticLinkStatus,
};
use tempfile::Builder;

#[test]
fn deck_runtime_native_static_link_plan_rejects_non_ready_object_receipt() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-link-not-ready-")
        .tempdir()
        .expect("create root");

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gsc(root.path().join("missing-gsc"))
        .build_link_unit();
    let link_plan = receipt.static_link_plan();

    assert_eq!(
        link_plan.status,
        GerbilDeckRuntimeNativeStaticLinkStatus::LinkUnitNotReady
    );
    assert!(link_plan.cargo_directives.is_empty());
    assert!(
        link_plan
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("native Deck runtime link unit is not ready"))
    );
}
