"""Hermetic composition for a Rust crate guarded by the Marlin Harness policy."""

load("@rules_rust//cargo:defs.bzl", "cargo_build_script")
load("//bazel:marlin_rust.bzl", "marlin_rust_crate")

def marlin_harness_crate(
        name,
        srcs,
        deps,
        build_deps,
        edition,
        gate_data = [],
        manifest = "Cargo.toml",
        visibility = ["//visibility:public"],
        **kwargs):
    """Declares one library and its runtime source-graph build gate.

    The gate scans its package at execution time, so the manifest, library
    sources, and gate_data must be runtime data rather than compile_data.
    """
    build_action = name + "_build_action"

    runtime_source_graph = build_action + "_runtime_source_graph"

    native.filegroup(
        name = runtime_source_graph,
        srcs = [manifest] + srcs + gate_data + native.glob(
            ["tests/unit/scenarios/performance_baseline/*.toml"],
            allow_empty = True,
        ),
        visibility = ["//:__pkg__"],
    )

    cargo_build_script(
        name = build_action,
        srcs = ["build.rs"],
        data = [":" + runtime_source_graph],
        edition = edition,
        deps = build_deps,
    )

    marlin_rust_crate(
        name = name,
        srcs = srcs,
        build_action = "harness",
        build_script = ":" + build_action,
        crate_root = "src/lib.rs",
        deps = deps,
        edition = edition,
        manifest = manifest,
        visibility = visibility,
        **kwargs
    )
