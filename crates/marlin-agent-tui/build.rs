// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[cfg(feature = "linked-ui")]
fn main() {
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
        fn observe(&self, _observation: ProgramArchiveObservation<'_>) {}

        fn observe_source_input(&self, source: &Path) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }

    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let manifest = root.join("../../scheme/.gerbil/native/tui/program.json");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_observed(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: &gsc,
            archive_name: "marlin_tui",
            linker_name: "marlin_tui",
            out_dir: &out_dir,
        },
        &CargoSourceObserver,
    )
    .expect("build selected Marlin TUI programme archive; run just tui-aot-stage first");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}

#[cfg(not(feature = "linked-ui"))]
fn main() {}
