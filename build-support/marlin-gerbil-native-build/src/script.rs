//! Build-script implementation for Gerbil native integration crates.

use std::{env, path::PathBuf};

use crate::{
    archive::{
        AGENT_POLICY_ROUTING_NATIVE_ARCHIVE_NAME, DECK_RUNTIME_NATIVE_ARCHIVE_NAME,
        build_static_archive_from_link_plan,
    },
    discovery::{GambitLinkSearchDiscovery, find_gambit_link_search_dir_from_gsc},
    toolchain::discover_native_c_compiler,
};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandReceipt,
    GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeAotProfile,
    GerbilDeckRuntimeNativeStaticLinkPlan, GerbilDeckRuntimeNativeStaticLinkStatus,
    default_gerbil_gsc_program,
};

const DECK_C_COMPILER_ENV: &str = "MARLIN_DECK_RUNTIME_NATIVE_C_COMPILER";
const DECK_GAMBIT_LINK_LIBRARY_ENV: &str = "MARLIN_DECK_RUNTIME_NATIVE_GAMBIT_LINK_LIBRARY";
const DECK_GAMBIT_LINK_SEARCH_DIR_ENV: &str = "MARLIN_DECK_RUNTIME_NATIVE_GAMBIT_LINK_SEARCH_DIR";
const AGENT_C_COMPILER_ENV: &str = "MARLIN_AGENT_POLICY_ROUTING_NATIVE_C_COMPILER";
const AGENT_GAMBIT_LINK_LIBRARY_ENV: &str =
    "MARLIN_AGENT_POLICY_ROUTING_NATIVE_GAMBIT_LINK_LIBRARY";
const AGENT_GAMBIT_LINK_SEARCH_DIR_ENV: &str =
    "MARLIN_AGENT_POLICY_ROUTING_NATIVE_GAMBIT_LINK_SEARCH_DIR";

/// Emits native Deck runtime link directives when the explicit feature is enabled.
pub fn emit_deck_runtime_native_link_directives() {
    emit_native_link_directives_for_target(NativeLinkTarget::deck_runtime());
}

/// Emits native AgentGraph policy-routing link directives when the explicit feature is enabled.
pub fn emit_agent_policy_routing_native_link_directives() {
    emit_native_link_directives_for_target(NativeLinkTarget::agent_policy_routing());
}

fn emit_native_link_directives_for_target(target: NativeLinkTarget) {
    emit_native_link_rerun_inputs(target);
    if env::var_os("CARGO_FEATURE_LINKED_NATIVE").is_some() {
        emit_native_link_directives(target);
    }
}

fn emit_native_link_rerun_inputs(target: NativeLinkTarget) {
    for name in [
        target.c_compiler_env,
        target.gambit_link_library_env,
        target.gambit_link_search_dir_env,
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }

    for name in [
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

fn emit_native_link_directives(target: NativeLinkTarget) {
    let root = out_dir().join(target.link_root_dir);
    let mut config = GerbilDeckRuntimeNativeAotConfig::new_for_profile(&root, target.profile);
    if let Some(c_compiler) = env::var_os(target.c_compiler_env) {
        config = config.with_c_compiler(c_compiler.to_string_lossy());
    } else if let Ok(c_compiler) = discover_native_c_compiler() {
        config = config.with_c_compiler(c_compiler.program.to_string_lossy());
    }
    if let Some(link_library) = env::var_os(target.gambit_link_library_env) {
        config = config.with_gambit_link_library(link_library.to_string_lossy());
    }
    if let Some(search_dir) = env::var_os(target.gambit_link_search_dir_env) {
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
                "cargo:warning=native {} link unit skipped during Clippy: {}",
                target.label,
                native_link_failure_detail(&receipt, &link_plan)
            );
            return;
        }
        panic!(
            "native {} link unit is not ready: {}",
            target.label,
            native_link_failure_detail(&receipt, &link_plan)
        );
    }

    let archive_dir = out_dir().join(target.archive_dir);
    match build_static_archive_from_link_plan(target.archive_name, &link_plan, &archive_dir) {
        Ok(archive) => {
            println!("cargo:rerun-if-changed={}", archive.archive_file.display());
            for directive in archive.cargo_directives {
                println!("{}", directive.line());
            }
        }
        Err(error) => {
            println!(
                "cargo:warning=native {} archive packaging failed; falling back to object link args: {error}",
                target.label
            );
            for directive in link_plan.cargo_directives {
                println!("{}", directive.line());
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeLinkTarget {
    profile: GerbilDeckRuntimeNativeAotProfile,
    label: &'static str,
    link_root_dir: &'static str,
    archive_dir: &'static str,
    archive_name: &'static str,
    c_compiler_env: &'static str,
    gambit_link_library_env: &'static str,
    gambit_link_search_dir_env: &'static str,
}

impl NativeLinkTarget {
    const fn deck_runtime() -> Self {
        Self {
            profile: GerbilDeckRuntimeNativeAotProfile::DeckRuntime,
            label: "Deck runtime",
            link_root_dir: "deck-runtime-native-link-root",
            archive_dir: "deck-runtime-native-link-archive",
            archive_name: DECK_RUNTIME_NATIVE_ARCHIVE_NAME,
            c_compiler_env: DECK_C_COMPILER_ENV,
            gambit_link_library_env: DECK_GAMBIT_LINK_LIBRARY_ENV,
            gambit_link_search_dir_env: DECK_GAMBIT_LINK_SEARCH_DIR_ENV,
        }
    }

    const fn agent_policy_routing() -> Self {
        Self {
            profile: GerbilDeckRuntimeNativeAotProfile::AgentPolicyRouting,
            label: "AgentGraph policy-routing",
            link_root_dir: "agent-policy-routing-native-link-root",
            archive_dir: "agent-policy-routing-native-link-archive",
            archive_name: AGENT_POLICY_ROUTING_NATIVE_ARCHIVE_NAME,
            c_compiler_env: AGENT_C_COMPILER_ENV,
            gambit_link_library_env: AGENT_GAMBIT_LINK_LIBRARY_ENV,
            gambit_link_search_dir_env: AGENT_GAMBIT_LINK_SEARCH_DIR_ENV,
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
