use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_deck_runtime_native_build::{
    DECK_RUNTIME_NATIVE_ARCHIVE_NAME, build_static_archive_from_link_plan,
    discover_native_c_compiler, static_archive_cargo_directives, static_archive_file_name,
};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeStaticLinkPlan, GerbilDeckRuntimeNativeStaticLinkStatus,
    GerbilNativeLinkLibrary,
};

#[test]
fn static_archive_filename_matches_target_family() {
    let file_name = static_archive_file_name(DECK_RUNTIME_NATIVE_ARCHIVE_NAME);

    if cfg!(target_env = "msvc") {
        assert_eq!(file_name, "marlin_deck_runtime_native.lib");
    } else {
        assert_eq!(file_name, "libmarlin_deck_runtime_native.a");
    }
}

#[test]
fn static_archive_directives_keep_archive_before_gambit_runtime() {
    let archive_dir = PathBuf::from("/tmp/marlin-deck-runtime-archive");
    let gambit_dir = PathBuf::from("/opt/gambit/lib");
    let plan = GerbilDeckRuntimeNativeStaticLinkPlan {
        status: GerbilDeckRuntimeNativeStaticLinkStatus::Ready,
        module_object: PathBuf::from("deck-runtime-native~0.o"),
        link_object: PathBuf::from("deck-runtime-native~0_.o"),
        header: PathBuf::from("marlin_deck_runtime_native.h"),
        required_symbols: Vec::new(),
        link_libraries: vec![GerbilNativeLinkLibrary::new("gambit")],
        link_search_dirs: vec![gambit_dir.clone()],
        cargo_directives: Vec::new(),
        detail: None,
    };

    let directives =
        static_archive_cargo_directives(DECK_RUNTIME_NATIVE_ARCHIVE_NAME, &archive_dir, &plan);
    let lines = directives
        .iter()
        .map(|directive| directive.line())
        .collect::<Vec<_>>();

    assert_eq!(
        lines,
        vec![
            format!("cargo:rustc-link-search=native={}", archive_dir.display()),
            "cargo:rustc-link-lib=static=marlin_deck_runtime_native".to_string(),
            format!("cargo:rustc-link-search=native={}", gambit_dir.display()),
            "cargo:rustc-link-lib=gambit".to_string(),
        ]
    );
}

#[test]
#[cfg(unix)]
fn static_archive_packaging_consumes_real_object_files() {
    let Ok(compiler) = discover_native_c_compiler() else {
        return;
    };
    let root = unique_temp_dir("marlin-deck-runtime-native-build-archive");
    let objects = root.join("objects");
    let archive_dir = root.join("archive");
    fs::create_dir_all(&objects).expect("create object dir");
    fs::write(
        objects.join("module.c"),
        "int marlin_deck_runtime_archive_module(void) { return 1; }\n",
    )
    .expect("write module source");
    fs::write(
        objects.join("link.c"),
        "int marlin_deck_runtime_archive_link(void) { return 2; }\n",
    )
    .expect("write link source");

    let module_object = objects.join("module.o");
    let link_object = objects.join("link.o");
    compile_c_object(&compiler.program, &objects.join("module.c"), &module_object);
    compile_c_object(&compiler.program, &objects.join("link.c"), &link_object);

    let plan = GerbilDeckRuntimeNativeStaticLinkPlan {
        status: GerbilDeckRuntimeNativeStaticLinkStatus::Ready,
        module_object,
        link_object,
        header: PathBuf::from("marlin_deck_runtime_native.h"),
        required_symbols: Vec::new(),
        link_libraries: Vec::new(),
        link_search_dirs: Vec::new(),
        cargo_directives: Vec::new(),
        detail: None,
    };

    let receipt = build_static_archive_from_link_plan(&plan, &archive_dir)
        .expect("package object files into a static archive");

    assert_eq!(receipt.archive_name, DECK_RUNTIME_NATIVE_ARCHIVE_NAME);
    assert!(receipt.archive_file.is_file());
    assert!(
        receipt
            .cargo_directives
            .iter()
            .any(|directive| directive.line()
                == "cargo:rustc-link-lib=static=marlin_deck_runtime_native")
    );

    fs::remove_dir_all(root).expect("remove archive temp dir");
}

#[cfg(unix)]
fn compile_c_object(compiler: &Path, source: &Path, object: &Path) {
    let status = Command::new(compiler)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(object)
        .status()
        .expect("run C compiler");
    assert!(
        status.success(),
        "C compiler failed for {}",
        source.display()
    );
    assert!(object.is_file(), "missing object {}", object.display());
}

#[cfg(unix)]
fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
