"""Exposes a Rust crate and the source data used by development policy tests."""

load("//bazel:marlin_rust.bzl", "marlin_rust_crate")

def marlin_crate_with_policy_data(
        name,
        srcs,
        deps,
        edition,
        gate_data = [],
        manifest = "Cargo.toml",
        visibility = ["//visibility:public"],
        **kwargs):
    """Declares one library and the source graph scanned by policy tests."""

    native.filegroup(
        name = name + "_runtime_source_graph",
        srcs = [manifest] + srcs + gate_data + native.glob(
            ["tests/unit/scenarios/performance_baseline/*.toml"],
            allow_empty = True,
        ),
        visibility = ["//:__pkg__"],
    )

    marlin_rust_crate(
        name = name,
        srcs = srcs,
        build_action = "none",
        crate_root = "src/lib.rs",
        deps = deps,
        edition = edition,
        manifest = manifest,
        visibility = visibility,
        **kwargs
    )
