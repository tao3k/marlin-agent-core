// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[cfg(feature = "linked-ui")]
fn main() {
    use gerbil_scheme_aot_build::{
        ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
        ProgramArchiveRequest, build_program_archive_with_contract, discover_gambit_gsc_from_env,
    };
    use std::{
        env,
        num::NonZeroUsize,
        path::{Path, PathBuf},
    };

    struct CargoSourceObserver;

    impl ProgramArchiveObserver for CargoSourceObserver {
        fn observe(&self, _observation: ProgramArchiveObservation<'_>) {}

        fn observe_source_input(&self, source: &Path) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let manifest = marlin_gerbil_aot_link::scheme_link_manifest(
        marlin_gerbil_aot_link::SchemeProgram::new("ui-tui"),
        &out_dir.join("program"),
    )
    .expect("read the ui-tui link products; run just scheme-build first");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_with_contract(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: &gsc,
            archive_name: "marlin_tui",
            linker_name: "marlin_tui",
            out_dir: &out_dir,
        },
        ProgramArchiveContract {
            // Bound the complete selected Scheme closure, including metadata.
            // Keep this admission budget in the host rather than producer metadata.
            max_modules: NonZeroUsize::new(2048).expect("nonzero module budget"),
            required_modules: &["gerbil-scheme-rust/scheme/runtime"],
            forbidden_modules: &[],
            linker_main_symbol: "gerbil_scheme_rust_program_main",
            additional_objects: &[],
            native_headers: &[],
        },
        &CargoSourceObserver,
    )
    .expect("link the Interface-selected Marlin TUI programme");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}

#[cfg(not(feature = "linked-ui"))]
fn main() {}
