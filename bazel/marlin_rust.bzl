"""Typed, low-fanout Bazel contracts for Marlin Rust packages."""

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

_BUILD_ACTIONS = [
    "harness",
    "generated+harness",
    "native+harness",
]

_SCENARIOS = [
    "library",
    "binary",
    "unit",
    "integration",
    "storage_default",
    "storage_turso",
    "storage_turso_sync",
    "turso",
    "turso_sync",
    "policy_gate",
    "cargo_parity",
]

_OPTIMIZATIONS = [
    "development",
    "speed",
    "size",
]

_OPTIMIZATION_RUSTC_FLAGS = {
    "development": [],
    "size": ["-Copt-level=z"],
    "speed": ["-Copt-level=3"],
}

MarlinCrateContractInfo = provider(
    doc = "A queryable Marlin crate, feature, scenario, and build-action contract.",
    fields = {
        "build_action": "The composed build action classification.",
        "features": "The exact Cargo feature set represented by the target.",
        "manifest": "The Cargo manifest owning the target.",
        "optimization": "The explicit Rust optimization contract.",
        "receipt": "The emitted JSON contract receipt.",
        "scenario": "The explicit build or test scenario.",
        "target": "The Bazel target governed by this contract.",
    },
)

def _crate_contract_impl(ctx):
    features = sorted(ctx.attr.crate_features)
    receipt = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(
        output = receipt,
        content = json.encode({
            "build_action": ctx.attr.build_action,
            "features": features,
            "manifest": ctx.file.manifest.short_path,
            "optimization": ctx.attr.optimization,
            "scenario": ctx.attr.scenario,
            "schema": "marlin.bazel.crate-contract",
            "target": str(ctx.attr.target.label),
        }) + "\n",
    )
    return [
        DefaultInfo(files = depset([receipt])),
        OutputGroupInfo(marlin_contracts = depset([receipt])),
        MarlinCrateContractInfo(
            build_action = ctx.attr.build_action,
            features = features,
            manifest = ctx.file.manifest,
            optimization = ctx.attr.optimization,
            receipt = receipt,
            scenario = ctx.attr.scenario,
            target = ctx.attr.target.label,
        ),
    ]

_crate_contract = rule(
    implementation = _crate_contract_impl,
    attrs = {
        "build_action": attr.string(
            mandatory = True,
            values = _BUILD_ACTIONS,
        ),
        "crate_features": attr.string_list(),
        "manifest": attr.label(
            allow_single_file = ["Cargo.toml"],
            mandatory = True,
        ),
        "optimization": attr.string(
            mandatory = True,
            values = _OPTIMIZATIONS,
        ),
        "scenario": attr.string(
            mandatory = True,
            values = _SCENARIOS,
        ),
        "target": attr.label(mandatory = True),
    },
)

def _contract_target_key(contract):
    return contract["target"]

def _contract_set_impl(ctx):
    contracts = []
    receipts = []
    for dependency in ctx.attr.contracts:
        contract = dependency[MarlinCrateContractInfo]
        contracts.append({
            "build_action": contract.build_action,
            "features": contract.features,
            "manifest": contract.manifest.short_path,
            "optimization": contract.optimization,
            "scenario": contract.scenario,
            "target": str(contract.target),
        })
        receipts.append(contract.receipt)
    contracts = sorted(contracts, key = _contract_target_key)
    receipt = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(
        output = receipt,
        content = json.encode({
            "contract_count": len(contracts),
            "contracts": contracts,
            "schema": "marlin.bazel.workspace-contract-set",
        }) + "\n",
    )
    return [
        DefaultInfo(files = depset([receipt] + receipts)),
        OutputGroupInfo(marlin_contracts = depset([receipt] + receipts)),
    ]

marlin_contract_set = rule(
    implementation = _contract_set_impl,
    attrs = {
        "contracts": attr.label_list(
            mandatory = True,
            providers = [MarlinCrateContractInfo],
        ),
    },
)

def _marlin_rust_crate_impl(
        name,
        visibility,
        build_action,
        build_script,
        crate_features,
        deps,
        manifest,
        optimization,
        rustc_flags,
        scenario,
        **kwargs):
    rust_library(
        name = name,
        visibility = visibility,
        crate_features = crate_features,
        deps = deps + ([build_script] if build_script != None else []),
        rustc_flags = rustc_flags + _OPTIMIZATION_RUSTC_FLAGS[optimization],
        **kwargs
    )
    _crate_contract(
        name = name + "_contract",
        visibility = visibility,
        build_action = build_action,
        crate_features = crate_features,
        manifest = manifest,
        optimization = optimization,
        scenario = scenario,
        target = name,
    )

marlin_rust_crate = macro(
    doc = "Declares one Rust library and one typed contract receipt; it creates no implicit tests.",
    inherit_attrs = rust_library,
    implementation = _marlin_rust_crate_impl,
    attrs = {
        "build_action": attr.string(
            default = "harness",
            values = _BUILD_ACTIONS,
            configurable = False,
        ),
        "build_script": attr.label(configurable = False),
        "crate_features": attr.string_list(configurable = False),
        "deps": attr.label_list(configurable = False),
        "manifest": attr.label(
            allow_single_file = ["Cargo.toml"],
            mandatory = True,
            configurable = False,
        ),
        "optimization": attr.string(
            default = "development",
            values = _OPTIMIZATIONS,
            configurable = False,
        ),
        "rustc_flags": attr.string_list(configurable = False),
        "scenario": attr.string(
            default = "library",
            values = ["library", "turso", "turso_sync"],
            configurable = False,
        ),
    },
)

def _marlin_rust_binary_impl(
        name,
        visibility,
        build_action,
        build_script,
        crate_features,
        deps,
        manifest,
        optimization,
        rustc_flags,
        **kwargs):
    rust_binary(
        name = name,
        visibility = visibility,
        crate_features = crate_features,
        deps = deps + ([build_script] if build_script != None else []),
        rustc_flags = rustc_flags + _OPTIMIZATION_RUSTC_FLAGS[optimization],
        **kwargs
    )
    _crate_contract(
        name = name + "_contract",
        visibility = visibility,
        build_action = build_action,
        crate_features = crate_features,
        manifest = manifest,
        optimization = optimization,
        scenario = "binary",
        target = name,
    )

marlin_rust_binary = macro(
    doc = "Declares one Rust binary and one typed contract receipt.",
    inherit_attrs = rust_binary,
    implementation = _marlin_rust_binary_impl,
    attrs = {
        "build_action": attr.string(
            default = "harness",
            values = _BUILD_ACTIONS,
            configurable = False,
        ),
        "build_script": attr.label(configurable = False),
        "crate_features": attr.string_list(configurable = False),
        "deps": attr.label_list(configurable = False),
        "manifest": attr.label(
            allow_single_file = ["Cargo.toml"],
            mandatory = True,
            configurable = False,
        ),
        "optimization": attr.string(
            default = "development",
            values = _OPTIMIZATIONS,
            configurable = False,
        ),
        "rustc_flags": attr.string_list(configurable = False),
    },
)

def _marlin_rust_scenario_impl(
        name,
        visibility,
        build_action,
        crate_features,
        deps,
        manifest,
        optimization,
        rustc_flags,
        scenario,
        **kwargs):
    rust_test(
        name = name,
        visibility = visibility,
        crate_features = crate_features,
        deps = deps,
        rustc_flags = rustc_flags + _OPTIMIZATION_RUSTC_FLAGS[optimization],
        **kwargs
    )
    _crate_contract(
        name = name + "_contract",
        testonly = True,
        visibility = visibility,
        build_action = build_action,
        crate_features = crate_features,
        manifest = manifest,
        optimization = optimization,
        scenario = scenario,
        target = name,
    )

marlin_rust_scenario = macro(
    doc = "Declares exactly one explicit Rust test scenario and its typed receipt.",
    inherit_attrs = rust_test,
    implementation = _marlin_rust_scenario_impl,
    attrs = {
        "build_action": attr.string(
            default = "harness",
            values = _BUILD_ACTIONS,
            configurable = False,
        ),
        "crate_features": attr.string_list(configurable = False),
        "deps": attr.label_list(configurable = False),
        "manifest": attr.label(
            allow_single_file = ["Cargo.toml"],
            mandatory = True,
            configurable = False,
        ),
        "optimization": attr.string(
            default = "development",
            values = _OPTIMIZATIONS,
            configurable = False,
        ),
        "rustc_flags": attr.string_list(configurable = False),
        "scenario": attr.string(
            mandatory = True,
            values = _SCENARIOS,
            configurable = False,
        ),
    },
)
