// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Link products for the native Brief-to-Draft integration suite.

use std::{env, path::PathBuf};

use gerbil_scheme_aot_build::{
    ProgramArchiveRequest, build_program_archive_observed, discover_gambit_gsc_from_env,
};

use crate::archive::{CargoSourceObserver, SchemeProgram, scheme_link_manifest};

/// Link the Brief-to-Draft program published by the Gerbil package build.
pub fn emit_brief_to_draft_link_directives() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let manifest = scheme_link_manifest(
        SchemeProgram::new("brief-to-draft"),
        &out_dir.join("program"),
    )
    .expect("read the Brief-to-Draft link products; run just scheme-build first");
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_observed(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: &gsc,
            archive_name: "marlin_brief_to_draft",
            linker_name: "marlin_brief_to_draft",
            out_dir: &out_dir,
        },
        &CargoSourceObserver,
    )
    .expect("link the selected Brief-to-Draft Scheme graph");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}
