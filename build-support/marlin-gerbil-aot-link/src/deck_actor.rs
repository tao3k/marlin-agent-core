use std::{env, fs};

use gerbil_scheme_aot_build::{
    ProgramArchiveRequest, build_program_archive, discover_gambit_gsc_from_env, gerbil_command,
    run_native_process,
};

/// Link the compiled Deck Actor and rooted-object exports into the application.
pub fn emit_deck_actor_link_directives() {
    let scheme = marlin_gerbil_scheme::gerbil_package_root();
    let out = std::path::PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let lib = out.join("lib");
    let stage = out.join("stage");
    fs::create_dir_all(&stage).expect("create Deck AOT stage");
    println!("cargo:rerun-if-changed={}", scheme.display());
    for variable in [
        "GERBIL_HOME",
        "GERBIL_PATH",
        "GERBIL_LOADPATH",
        "GERBIL_GXI",
        "GERBIL_GSC",
        "PKG_CONFIG_PATH",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let mut stage_program = gerbil_command("gxi");
    let previous = env::var_os("GERBIL_LOADPATH").unwrap_or_default();
    let load_path = env::join_paths(
        std::iter::once(lib.clone())
            .chain(env::split_paths(&previous).filter(|path| !path.as_os_str().is_empty())),
    )
    .expect("Deck AOT Gerbil load path");
    stage_program.current_dir(&scheme).env("GERBIL_LOADPATH", load_path).env("MARLIN_DECK_AOT_LIB", &lib).env("MARLIN_DECK_AOT_STAGE", &stage).args([
        "-e",
        r#"(import :gerbil/compiler :gerbil-scheme-rust/scheme/program-build)
           (def source (path-expand "c-abi/deck-route-admission.ss"))
           (compile-module source [output-dir: (getenv "MARLIN_DECK_AOT_LIB") invoke-gsc: #f optimize: #t static: #t])
           (gerbil-rs-stage-program source (getenv "MARLIN_DECK_AOT_STAGE"))"#,
    ]);
    run_native_process(&mut stage_program, "stage Deck Actor Scheme graph", true)
        .expect("stage the Deck Actor graph from the compiled Scheme package and its dependencies");

    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive(ProgramArchiveRequest {
        manifest: &stage.join("program.json"),
        gsc: &gsc,
        archive_name: "marlin_deck_actor",
        linker_name: "marlin_deck_actor",
        out_dir: &out.join("archive"),
    })
    .expect("link the Deck Actor Scheme graph");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}
