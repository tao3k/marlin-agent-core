#[test]
fn scheme_binding_catalog_separates_upstream_backed_shapes_from_marlin_fixtures() {
    let receipt = marlin_gerbil_scheme::gerbil_scheme_backed_binding_catalog();

    assert_eq!(receipt.upstream_crate, "gerbil-scheme");
    assert_eq!(receipt.backed_shape_selectors.len(), 9);

    for selector in [
        "gerbil_scheme_rust_i64_shape",
        "gerbil_scheme_rust_bool_shape",
        "gerbil_scheme_rust_comparison_shape",
        "gerbil_scheme_rust_utf8_shape",
        "gerbil_scheme_rust_value_handle_shape",
        "gerbil_scheme_rust_i64_callback_shape",
        "gerbil_scheme_rust_native_value_shape",
        "gerbil_scheme_rust_native_error_shape",
        "gerbil_scheme_rust_native_result_shape",
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
            receipt
                .fixture_only_value_families
                .iter()
                .any(|candidate| candidate == fixture_only),
            "missing Marlin fixture-only value family: {fixture_only}"
        );
    }
}
