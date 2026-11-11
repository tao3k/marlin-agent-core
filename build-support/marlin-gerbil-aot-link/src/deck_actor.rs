use std::{env, fs};

use gerbil_scheme_aot_build::{
    ProgramArchiveRequest, build_program_archive, discover_gambit_gsc_from_env, gerbil_command,
    run_native_process, source_workspace,
};

/// Link the compiled Deck Actor and rooted-object exports into the application.
pub fn emit_deck_actor_link_directives() {
    let scheme = marlin_gerbil_scheme::gerbil_package_root();
    let out = std::path::PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let lib = out.join("lib");
    let stage = out.join("stage");
    let bridge = out.join("bridge");
    fs::create_dir_all(&stage).expect("create Deck AOT stage");
    println!("cargo:rerun-if-changed={}", scheme.display());
    for variable in [
        "GERBIL_HOME",
        "GERBIL_PATH",
        "GERBIL_LOADPATH",
        "GERBIL_GXI",
        "GERBIL_GSC",
        "GERBIL_BUILD_CORES",
        "GAMBOPT",
        "LDFLAGS_GAMBUILD",
        "PKG_CONFIG_PATH",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    // The Scheme bridge graph must come from the same Cargo-pinned source as
    // the Rust ABI. A transitive Lab package may select a different revision.
    let source = source_workspace();
    let mut build_bridge = gerbil_command("gxi");
    build_bridge
        .current_dir(&source)
        .env("GERBIL_PATH", &bridge)
        .arg(source.join("build.ss"))
        .arg("compile");
    run_native_process(&mut build_bridge, "build the pinned Scheme bridge", true)
        .expect("build the canonical bridge package for the linked Actor VM");

    let package_prefix = env::var_os("GERBIL_PATH")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scheme.join(".gerbil"));
    let mut stage_program = gerbil_command("gxi");
    let previous = env::var_os("GERBIL_LOADPATH").unwrap_or_default();
    let load_path = env::join_paths(
        [lib.clone(), bridge.join("lib"), package_prefix.join("lib")]
            .into_iter()
            .chain(env::split_paths(&previous).filter(|path| !path.as_os_str().is_empty())),
    )
    .expect("Deck AOT Gerbil load path");
    let mut build_package = gerbil_command("gxi");
    build_package
        .current_dir(&scheme)
        .env("GERBIL_LOADPATH", &load_path)
        .env("GERBIL_PATH", &package_prefix)
        .arg(scheme.join("build.ss"))
        .arg("compile");
    run_native_process(&mut build_package, "build the Marlin Scheme modules", true)
        .expect("build Marlin's Yuanxu-selected modules through its standard Builder");

    let mailbox_benchmark = env::var_os("CARGO_FEATURE_MAILBOX_BENCHMARK").is_some();
    stage_program
        .current_dir(&scheme)
        .env("GERBIL_PATH", &package_prefix)
        .env("GERBIL_LOADPATH", load_path)
        .env("MARLIN_DECK_AOT_LIB", &lib)
        .env("MARLIN_DECK_AOT_STAGE", &stage)
        .env(
            "MARLIN_DECK_MAILBOX_BENCHMARK",
            if mailbox_benchmark { "1" } else { "0" },
        )
        .args([
        "-e",
        r#"(import :gerbil/compiler :gerbil-scheme-rust/scheme/program-build)
           (def benchmark? (equal? (getenv "MARLIN_DECK_MAILBOX_BENCHMARK") "1"))
           (when benchmark?
             (for-each (lambda (path)
               (compile-module (path-expand path)
                 [output-dir: (getenv "MARLIN_DECK_AOT_LIB") invoke-gsc: #f optimize: #t static: #t]))
               ["bench/mailbox.ss" "bench/compiler-roots.ss" "bench/context-profile.ss"]))
           (def source (path-expand (if benchmark? "bench/mailbox-entry.ss" "c-abi/deck-runtime.ss")))
           (compile-module source [output-dir: (getenv "MARLIN_DECK_AOT_LIB") invoke-gsc: #f optimize: #t static: #t])
           (gerbil-rs-stage-program source (getenv "MARLIN_DECK_AOT_STAGE") []
             (if benchmark? ["marlin-benchmark/mailbox" "marlin-agent-core/deck-runtime/mailbox"]
                             ["marlin-agent-core/deck-runtime/mailbox"]))"#,
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
