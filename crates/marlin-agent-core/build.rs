// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

fn main() {
    #[cfg(feature = "brief-to-draft-vm-test")]
    link_workflow_test();
}

#[cfg(feature = "brief-to-draft-vm-test")]
fn link_workflow_test() {
    use gerbil_scheme_aot_build::{
        ProgramArchiveObservation, ProgramArchiveObserver, ProgramArchiveRequest,
        build_program_archive_observed, discover_gambit_gsc_from_env,
    };
    use std::{
        env,
        path::{Path, PathBuf},
    };

    struct CargoSourceObserver;

    impl ProgramArchiveObserver for CargoSourceObserver {
        fn observe(&self, _: ProgramArchiveObservation<'_>) {}

        fn observe_source_input(&self, source: &Path) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }

    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let manifest = marlin_gerbil_aot_link::project_scheme_program(
        &root.join("../../scheme"),
        marlin_gerbil_aot_link::ProgramSelection::new(&["Marlin", "brief-to-draft"])
            .expect("workflow Module selection"),
        &out_dir.join("program"),
    )
    .expect("project the built Brief-to-Draft Module through its Interface");
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
