// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    env,
    path::{Path, PathBuf},
};

use gerbil_scheme_aot_build::{
    ProgramArchiveObservation, ProgramArchiveObserver, ProgramArchiveRequest,
    build_program_archive_observed, discover_gambit_gsc_from_env,
};

use crate::archive::{SchemeProgram, scheme_link_manifest_with_layout};

/// Link the Deck program published by the standard Gerbil package build.
pub fn emit_deck_actor_link_directives() {
    struct CargoSourceObserver;

    impl ProgramArchiveObserver for CargoSourceObserver {
        fn observe(&self, _: ProgramArchiveObservation<'_>) {}

        fn observe_source_input(&self, source: &Path) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let (program, separate_procedures): (_, &[_]) =
        if env::var_os("CARGO_FEATURE_MAILBOX_BENCHMARK").is_some() {
            (
                "bench",
                &[
                    "marlin-agent-core/bench/mailbox",
                    "marlin-agent-core/deck-runtime/mailbox",
                ],
            )
        } else {
            ("deck-runtime", &["marlin-agent-core/deck-runtime/mailbox"])
        };
    let manifest = scheme_link_manifest_with_layout(
        SchemeProgram::new(program),
        &out.join("program"),
        separate_procedures,
    )
    .expect("read the Deck link products; run just scheme-build first");
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_observed(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: &gsc,
            archive_name: "marlin_deck_actor",
            linker_name: "marlin_deck_actor",
            out_dir: &out.join("archive"),
        },
        &CargoSourceObserver,
    )
    .expect("link the Deck Actor Scheme graph");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}
