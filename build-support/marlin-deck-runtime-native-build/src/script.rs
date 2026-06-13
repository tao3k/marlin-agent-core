//! Build-script implementation for the native Deck runtime integration crate.

use std::{env, path::PathBuf};

use crate::{
    archive::build_static_archive_from_link_plan,
    discovery::{GambitLinkSearchDiscovery, find_gambit_link_search_dir_from_gsc},
    toolchain::discover_native_c_compiler,
};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandReceipt,
    GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeStaticLinkPlan,
    GerbilDeckRuntimeNativeStaticLinkStatus, default_gerbil_gsc_program,
};

const GAMBIT_LINK_SEARCH_DIR_ENV: &str = "MARLIN_DECK_RUNTIME_NATIVE_GAMBIT_LINK_SEARCH_DIR";

/// Emits native Deck runtime link directives when the explicit feature is enabled.
pub fn emit_deck_runtime_native_link_directives() {
    emit_native_link_rerun_inputs();
    if env::var_os("CARGO_FEATURE_LINKED_NATIVE").is_some() {
        emit_native_link_directives();
    }
}

fn emit_native_link_rerun_inputs() {
    for name in [
        "MARLIN_DECK_RUNTIME_NATIVE_C_COMPILER",
        "MARLIN_DECK_RUNTIME_NATIVE_GAMBIT_LINK_LIBRARY",
        GAMBIT_LINK_SEARCH_DIR_ENV,
        "AR",
        "CC",
        "CFLAGS",
        "GAMBOPT",
        "GERBIL_HOME",
        "HOST_AR",
        "HOST_CC",
        "MARLIN_GERBIL_GSC",
        "MARLIN_GERBIL_GXC",
        "TARGET_AR",
        "TARGET_CC",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}

fn emit_native_link_directives() {
    let root = out_dir().join("deck-runtime-native-link-root");
    let mut config = GerbilDeckRuntimeNativeAotConfig::new(&root);
    if let Some(c_compiler) = env::var_os("MARLIN_DECK_RUNTIME_NATIVE_C_COMPILER") {
        config = config.with_c_compiler(c_compiler.to_string_lossy());
    } else if let Ok(c_compiler) = discover_native_c_compiler() {
        config = config.with_c_compiler(c_compiler.program.to_string_lossy());
    }
    if let Some(link_library) = env::var_os("MARLIN_DECK_RUNTIME_NATIVE_GAMBIT_LINK_LIBRARY") {
        config = config.with_gambit_link_library(link_library.to_string_lossy());
    }
    if let Some(search_dir) = env::var_os(GAMBIT_LINK_SEARCH_DIR_ENV) {
        config = config.with_gambit_link_search_dir(search_dir);
    } else if let Some(discovery) = discover_gambit_link_search_dir() {
        println!(
            "cargo:rerun-if-changed={}",
            discovery.library_path.display()
        );
        config = config.with_gambit_link_search_dir(discovery.search_dir);
    }

    let receipt = config.build_link_unit();
    let link_plan = receipt.static_link_plan();
    if link_plan.status != GerbilDeckRuntimeNativeStaticLinkStatus::Ready {
        if running_under_clippy() {
            println!(
                "cargo:warning=native Deck runtime link unit skipped during Clippy: {}",
                native_link_failure_detail(&receipt, &link_plan)
            );
            return;
        }
        panic!(
            "native Deck runtime link unit is not ready: {}",
            native_link_failure_detail(&receipt, &link_plan)
        );
    }

    let archive_dir = out_dir().join("deck-runtime-native-link-archive");
    match build_static_archive_from_link_plan(&link_plan, &archive_dir) {
        Ok(archive) => {
            println!("cargo:rerun-if-changed={}", archive.archive_file.display());
            for directive in archive.cargo_directives {
                println!("{}", directive.line());
            }
        }
        Err(error) => {
            println!(
                "cargo:warning=native Deck runtime archive packaging failed; falling back to object link args: {error}"
            );
            for directive in link_plan.cargo_directives {
                println!("{}", directive.line());
            }
        }
    }
}

fn discover_gambit_link_search_dir() -> Option<GambitLinkSearchDiscovery> {
    find_gambit_link_search_dir_from_gsc(&default_gerbil_gsc_program())
}

fn out_dir() -> PathBuf {
    env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .expect("Cargo sets OUT_DIR for build scripts")
}

fn running_under_clippy() -> bool {
    env::var_os("RUSTC_WORKSPACE_WRAPPER")
        .or_else(|| env::var_os("RUSTC_WRAPPER"))
        .is_some_and(rustc_wrapper_is_clippy)
}

/// Returns true when a Rust compiler wrapper path points at `clippy-driver`.
pub fn rustc_wrapper_is_clippy(wrapper: impl AsRef<std::ffi::OsStr>) -> bool {
    std::path::Path::new(wrapper.as_ref())
        .file_name()
        .is_some_and(|file_name| file_name == "clippy-driver")
}

fn native_link_failure_detail(
    receipt: &GerbilDeckRuntimeNativeAotBuildReceipt,
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
) -> String {
    [
        format!("link_status={:?}", link_plan.status),
        format!("build_status={:?}", receipt.status),
        format!(
            "detail={:?}",
            link_plan.detail.as_ref().or(receipt.detail.as_ref())
        ),
        command_detail("gxc", receipt.gxc_generate_scheme.as_ref()),
        command_detail("gsc-object", receipt.gsc_compile_object.as_ref()),
        command_detail("gsc-link-source", receipt.gsc_generate_link_source.as_ref()),
        command_detail("gsc-link-object", receipt.gsc_compile_link_object.as_ref()),
        command_detail("symbol-audit", receipt.symbol_audit.as_ref()),
    ]
    .into_iter()
    .filter(|detail| !detail.is_empty())
    .collect::<Vec<_>>()
    .join("; ")
}

fn command_detail(
    label: &str,
    receipt: Option<&GerbilDeckRuntimeNativeAotCommandReceipt>,
) -> String {
    let Some(receipt) = receipt else {
        return String::new();
    };
    format!(
        "{label}=status:{:?} stdout:{:?} stderr:{:?}",
        receipt.status_code,
        receipt.stdout.trim(),
        receipt.stderr.trim()
    )
}
