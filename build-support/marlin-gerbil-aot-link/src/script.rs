//! Build-script implementation for Gerbil native integration crates.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use gerbil_scheme_build::package_aot::{GerbilBuildToolchain, run_package_aot_compile};

use crate::{
    archive::{
        AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME, DECK_ROUTE_ABI_ARCHIVE_NAME,
        build_static_archive_from_link_plan,
    },
    discovery::{GambitLinkSearchDiscovery, find_gambit_link_search_dir_from_gsc},
    toolchain::discover_c_compiler,
};
use marlin_gerbil_scheme::{
    GerbilDeckRouteAbiStaticLinkPlan, GerbilDeckRouteAbiStaticLinkStatus,
    GerbilDeckRuntimeAotBuildReceipt, GerbilDeckRuntimeAotCommandReceipt,
    GerbilDeckRuntimeAotConfig, GerbilDeckRuntimeAotProfile, default_gerbil_gsc_program,
    default_gerbil_gxc_program, default_gerbil_gxpkg_program, gerbil_package_root,
};

const DECK_C_COMPILER_ENV: &str = "MARLIN_DECK_ROUTE_ABI_C_COMPILER";
const DECK_GAMBIT_LINK_LIBRARY_ENV: &str = "MARLIN_DECK_ROUTE_ABI_GAMBIT_LINK_LIBRARY";
const DECK_GAMBIT_LINK_SEARCH_DIR_ENV: &str = "MARLIN_DECK_ROUTE_ABI_GAMBIT_LINK_SEARCH_DIR";
const AGENT_C_COMPILER_ENV: &str = "MARLIN_AGENT_POLICY_ROUTING_ABI_C_COMPILER";
const AGENT_GAMBIT_LINK_LIBRARY_ENV: &str = "MARLIN_AGENT_POLICY_ROUTING_ABI_GAMBIT_LINK_LIBRARY";
const AGENT_GAMBIT_LINK_SEARCH_DIR_ENV: &str =
    "MARLIN_AGENT_POLICY_ROUTING_ABI_GAMBIT_LINK_SEARCH_DIR";

/// Emits native Deck runtime link directives when the explicit feature is enabled.
pub fn emit_deck_route_abi_link_directives() {
    emit_aot_link_directives_for_target(AotLinkTarget::deck_runtime());
}

/// Emits native AgentGraph policy-routing link directives when the explicit feature is enabled.
pub fn emit_agent_policy_routing_abi_link_directives() {
    emit_aot_link_directives_for_target(AotLinkTarget::agent_policy_routing());
}

fn emit_aot_link_directives_for_target(target: AotLinkTarget) {
    emit_aot_link_rerun_inputs(target);
    if env::var_os("CARGO_FEATURE_LINKED_SELECTOR").is_some() {
        emit_aot_link_directives(target);
    }
}

fn emit_aot_link_rerun_inputs(target: AotLinkTarget) {
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
        "GERBIL_LOADPATH",
        "GERBIL_PATH",
        "HOST_AR",
        "HOST_CC",
        "MARLIN_GERBIL_GSC",
        "MARLIN_GERBIL_GXC",
        "MARLIN_GERBIL_GXPKG",
        "TARGET_AR",
        "TARGET_CC",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}

fn emit_aot_link_directives(target: AotLinkTarget) {
    let root = out_dir().join(target.link_root_dir);
    let compile_receipt = compile_aot_artifact(target, &root);
    if compile_receipt.status_code != Some(0) {
        if running_under_clippy() {
            println!(
                "cargo:warning=native {} package native compile skipped during Clippy: {}",
                target.label,
                command_detail("native-aot-compile", Some(&compile_receipt))
            );
            return;
        }
        panic!(
            "native {} package native compile failed: {}",
            target.label,
            command_detail("native-aot-compile", Some(&compile_receipt))
        );
    }

    let mut config = GerbilDeckRuntimeAotConfig::new_for_profile(&root, target.profile);
    if let Some(c_compiler) = env::var_os(target.c_compiler_env) {
        config = config.with_c_compiler(c_compiler.to_string_lossy());
    } else if let Ok(c_compiler) = discover_c_compiler() {
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
    if link_plan.status != GerbilDeckRouteAbiStaticLinkStatus::Ready {
        if running_under_clippy() {
            println!(
                "cargo:warning=native {} link unit skipped during Clippy: {}",
                target.label,
                aot_link_failure_detail(&receipt, &link_plan)
            );
            return;
        }
        panic!(
            "native {} link unit is not ready: {}",
            target.label,
            aot_link_failure_detail(&receipt, &link_plan)
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
struct AotLinkTarget {
    profile: GerbilDeckRuntimeAotProfile,
    source_path: &'static str,
    dependency_source_paths: &'static [&'static str],
    staged_scm_file: &'static str,
    staged_dependency_scm_files: &'static [&'static str],
    label: &'static str,
    link_root_dir: &'static str,
    archive_dir: &'static str,
    archive_name: &'static str,
    c_compiler_env: &'static str,
    gambit_link_library_env: &'static str,
    gambit_link_search_dir_env: &'static str,
}

impl AotLinkTarget {
    const fn deck_runtime() -> Self {
        Self {
            profile: GerbilDeckRuntimeAotProfile::DeckRuntime,
            source_path: "c-abi/deck-route-abi.ss",
            dependency_source_paths: &["c-abi/_deck-route-abi.ssi"],
            staged_scm_file: "deck-route-abi~0.scm",
            staged_dependency_scm_files: &["_deck-route-abi~0.scm"],
            label: "Deck runtime",
            link_root_dir: "deck-route-abi-link-root",
            archive_dir: "deck-route-abi-link-archive",
            archive_name: DECK_ROUTE_ABI_ARCHIVE_NAME,
            c_compiler_env: DECK_C_COMPILER_ENV,
            gambit_link_library_env: DECK_GAMBIT_LINK_LIBRARY_ENV,
            gambit_link_search_dir_env: DECK_GAMBIT_LINK_SEARCH_DIR_ENV,
        }
    }

    const fn agent_policy_routing() -> Self {
        Self {
            profile: GerbilDeckRuntimeAotProfile::AgentPolicyRouting,
            source_path: "c-abi/agent-policy-routing-abi.ss",
            dependency_source_paths: &["c-abi/_agent-policy-routing-abi.ssi"],
            staged_scm_file: "agent-policy-routing-abi~0.scm",
            staged_dependency_scm_files: &["_agent-policy-routing-abi~0.scm"],
            label: "AgentGraph policy-routing",
            link_root_dir: "agent-policy-routing-abi-link-root",
            archive_dir: "agent-policy-routing-abi-link-archive",
            archive_name: AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME,
            c_compiler_env: AGENT_C_COMPILER_ENV,
            gambit_link_library_env: AGENT_GAMBIT_LINK_LIBRARY_ENV,
            gambit_link_search_dir_env: AGENT_GAMBIT_LINK_SEARCH_DIR_ENV,
        }
    }
}

fn compile_aot_artifact(target: AotLinkTarget, root: &Path) -> GerbilDeckRuntimeAotCommandReceipt {
    let gerbil_pkg_dir = gerbil_package_root();
    let gerbil_pkg = gerbil_pkg_dir.join("gerbil.pkg");
    let source_path = gerbil_pkg_dir.join(target.source_path);
    println!("cargo:rerun-if-changed={}", source_path.display());
    for dependency_source_path in target.dependency_source_paths {
        println!(
            "cargo:rerun-if-changed={}",
            gerbil_pkg_dir.join(dependency_source_path).display()
        );
    }
    println!("cargo:rerun-if-changed={}", gerbil_pkg.display());

    let toolchain =
        GerbilBuildToolchain::new(default_gerbil_gxpkg_program(), default_gerbil_gxc_program());
    let builder_root = env::var_os("GERBIL_PATH")
        .filter(|value| !value.is_empty())
        .map_or_else(
            || gerbil_pkg_dir.join(".gerbil"),
            |path| gerbil_pkg_dir.join(path),
        );
    let compiled_dir = builder_root.join("lib/marlin-agent-core/c-abi");
    let stage_dir = root.join(".gerbil/native");
    let units = target
        .dependency_source_paths
        .iter()
        .copied()
        .zip(target.staged_dependency_scm_files.iter().copied())
        .chain(std::iter::once((
            target.source_path,
            target.staged_scm_file,
        )));
    let mut receipt = GerbilDeckRuntimeAotCommandReceipt {
        status_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
    };
    if let Err(error) = fs::create_dir_all(&stage_dir) {
        receipt.status_code = Some(67);
        receipt.stderr = format!(
            "create Gerbil link staging directory {}: {error}",
            stage_dir.display()
        );
        return receipt;
    }
    for (source, artifact) in units {
        let build = run_package_aot_compile(&toolchain, &gerbil_pkg_dir, source);
        receipt.status_code = build.status_code;
        receipt.stdout = build.stdout;
        receipt.stderr = build.stderr;
        if receipt.status_code != Some(0) {
            return receipt;
        }
        let compiled = compiled_dir.join(artifact);
        if let Err(error) = fs::copy(&compiled, stage_dir.join(artifact)) {
            receipt.status_code = Some(67);
            receipt.stderr = format!(
                "stage Gerbil compiler artifact {}: {error}",
                compiled.display()
            );
            return receipt;
        }
    }
    receipt
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

fn aot_link_failure_detail(
    receipt: &GerbilDeckRuntimeAotBuildReceipt,
    link_plan: &GerbilDeckRouteAbiStaticLinkPlan,
) -> String {
    let mut details = vec![
        format!("link_status={:?}", link_plan.status),
        format!("build_status={:?}", receipt.status),
        format!(
            "detail={:?}",
            link_plan.detail.as_ref().or(receipt.detail.as_ref())
        ),
        command_detail("gsc-object", receipt.gsc_compile_object.as_ref()),
    ];
    details.extend(
        receipt
            .gsc_compile_dependency_objects
            .iter()
            .enumerate()
            .map(|(index, receipt)| {
                command_detail(&format!("gsc-dependency-object-{index}"), Some(receipt))
            }),
    );
    details.extend([
        command_detail("gsc-link-source", receipt.gsc_generate_link_source.as_ref()),
        command_detail("gsc-link-object", receipt.gsc_compile_link_object.as_ref()),
        command_detail("symbol-audit", receipt.symbol_audit.as_ref()),
    ]);
    details
        .into_iter()
        .filter(|detail| !detail.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

fn command_detail(label: &str, receipt: Option<&GerbilDeckRuntimeAotCommandReceipt>) -> String {
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
