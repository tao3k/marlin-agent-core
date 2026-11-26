// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Link products for the Linked-AOT `marlin-scheme-host-aot` daemon binary.

use std::{env, num::NonZeroUsize, path::PathBuf};

use gerbil_scheme_aot_build::{
    ProgramArchiveContract, ProgramArchiveRequest, build_program_archive_with_contract,
    discover_gambit_gsc_from_env,
};

use crate::archive::{CargoSourceObserver, SchemeProgram, scheme_link_manifest};

/// Link the GraphEngine Oracle + Fleet Controller program published by `just scheme-build`.
pub fn emit_graph_engine_link_directives() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let manifest =
        scheme_link_manifest(SchemeProgram::new("graph-engine"), &out_dir.join("program"))
            .expect("read the graph-engine link products; run just scheme-build first");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_with_contract(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: &gsc,
            archive_name: "marlin_graph_engine",
            linker_name: "marlin_graph_engine",
            out_dir: &out_dir.join("archive"),
        },
        ProgramArchiveContract {
            max_modules: NonZeroUsize::new(2048).expect("nonzero module budget"),
            required_modules: &["gerbil-scheme-rust/scheme/runtime"],
            forbidden_modules: &[],
            linker_main_symbol: "gerbil_scheme_rust_program_main",
            additional_objects: &[],
            native_headers: &[],
        },
        &CargoSourceObserver,
    )
    .expect("link the selected GraphEngine Scheme graph");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}
