#[test]
fn scheme_binding_catalog_separates_upstream_backed_shapes_from_marlin_fixtures() {
    let receipt = marlin_gerbil_scheme::gerbil_scheme_backed_binding_catalog();

    assert_eq!(receipt.upstream_crate, "gerbil-scheme");
    assert!(receipt.backed_shape_selectors.len() >= 15);

    for selector in [
        "gerbil_scheme_rust_value_handle_shape",
        "gerbil_scheme_rust_value_is_pair",
        "gerbil_scheme_rust_value_is_list",
        "gerbil_scheme_rust_value_is_null",
        "gerbil_scheme_rust_pair_car",
        "gerbil_scheme_rust_pair_cdr",
        "gerbil_scheme_rust_pair_parts",
    ] {
        assert!(
            receipt
                .backed_shape_selectors
                .iter()
                .any(|candidate| candidate == selector),
            "missing upstream-backed binding shape selector: {selector}"
        );
    }

    for fixture_only in ["null-sentinel", "f64", "vector", "record"] {
        assert!(
            !receipt
                .backed_shape_selectors
                .iter()
                .any(|candidate| candidate == fixture_only),
            "Marlin fixture-only value family leaked into upstream-backed selector catalog: {fixture_only}"
        );
        assert!(
            receipt
                .fixture_only_value_families
                .iter()
                .any(|candidate| candidate == fixture_only),
            "missing Marlin fixture-only value family: {fixture_only}"
        );
    }
}
