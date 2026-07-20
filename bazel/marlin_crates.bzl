"""Stable facade for the generated Marlin Cargo dependency graph."""

load(
    "@marlin_crates//:defs.bzl",
    _all_crate_deps = "all_crate_deps",
    _crate_edition = "crate_edition",
)

all_crate_deps = _all_crate_deps
crate_edition = _crate_edition
