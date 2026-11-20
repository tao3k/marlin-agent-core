// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin archive adapters over the upstream Gerbil native build substrate.

use gerbil_scheme_aot_build::{
    CargoDirectiveKind, NativeStaticLinkPlan,
    build_static_archive_from_link_plan as build_upstream_static_archive, source_workspace,
    static_archive_cargo_directives as upstream_cargo_directives,
};
use marlin_gerbil_scheme::{
    GerbilAotCargoDirective, GerbilAotCargoDirectiveKind, GerbilAotStaticLinkPlan,
};
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
#[path = "../tests/unit/program_paths.rs"]
mod tests;

pub use gerbil_scheme_aot_build::static_archive_file_name;

/// Cargo/rustc link name for the generated AgentGraph policy-routing native archive.
pub const AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME: &str = "marlin_agent_policy_routing_abi";

/// The name of a program in the compiled Marlin module selection.
pub struct SchemeProgram<'a>(&'a str);

impl<'a> SchemeProgram<'a> {
    /// Use an existing Scheme module's published program entry.
    pub const fn new(module: &'a str) -> Self {
        Self(module)
    }
}

/// Consume the compiled package through the SDK and attach Cargo's ABI carriers.
///
/// # Errors
/// Returns an error when the package products, SDK carriers or link inputs are unavailable.
pub fn scheme_link_manifest(
    program: SchemeProgram<'_>,
    output: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    scheme_link_manifest_with_layout(program, output, &[])
}

pub(crate) fn scheme_link_manifest_with_layout(
    program: SchemeProgram<'_>,
    output: &Path,
    separate_procedures: &[&str],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    for variable in ["GERBIL_LOADPATH", "GERBIL_HOME"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    let load_path = env::var_os("GERBIL_LOADPATH").ok_or("GERBIL_LOADPATH is not set")?;
    let product = Path::new("marlin-agent-core/programs").join(program.0);
    let (library, published) = env::split_paths(&load_path)
        .filter(|path| !path.as_os_str().is_empty())
        .find_map(|library| {
            let published = library.join(&product).join("program.json");
            published.is_file().then_some((library, published))
        })
        .ok_or_else(|| {
            format!(
                "{} was not published on GERBIL_LOADPATH; run gerbil build",
                program.0
            )
        })?;
    let library = library.canonicalize()?;
    println!("cargo:rerun-if-changed={}", published.display());
    fs::create_dir_all(output)?;
    let mut plan: Value = serde_json::from_reader(fs::File::open(published)?)?;
    let sdk_origin = PathBuf::from(
        plan["library_dir"]
            .as_str()
            .ok_or("compiler program has no SDK library directory")?,
    );
    let sdk_library = env::var_os("GERBIL_HOME")
        .filter(|home| !home.is_empty())
        .map_or_else(
            || sdk_origin.clone(),
            |home| PathBuf::from(home).join("lib"),
        );
    let modules = rebase_program_paths(&mut plan, &product, &library, &sdk_origin, &sdk_library)?;
    for identity in separate_procedures {
        let module = modules
            .iter_mut()
            .find(|row| row["module"] == *identity)
            .ok_or_else(|| format!("separate-procedure module is absent: {identity}"))?;
        module["host_layout"] = json!("separate-procedures");
    }
    let sdk = source_workspace();
    let mut carriers = Vec::new();
    // The package is SDK-independent. Bind the same SDK units as the Rust host.
    for unit in ["utf8", "runtime"] {
        let identity = format!("gerbil-scheme-rust/scheme/{unit}");
        if modules.iter().any(|row| row["module"] == identity) {
            return Err(format!("Scheme module graph imports the Rust SDK: {identity}").into());
        }
        let scm = sdk.join(format!("scheme/generated/{unit}.scm"));
        if !scm.is_file() {
            return Err(format!("Cargo SDK carrier missing: {}", scm.display()).into());
        }
        println!("cargo:rerun-if-changed={}", scm.display());
        carriers.push(json!({"module": identity, "scm": scm, "system": false}));
    }
    let position = modules
        .iter()
        .position(|row| {
            !row["module"]
                .as_str()
                .is_some_and(|id| id.starts_with("gerbil/runtime/"))
        })
        .unwrap_or(modules.len());
    modules.splice(position..position, carriers);
    let manifest = output.join("linked-program.json");
    let bytes = serde_json::to_vec(&plan)?;
    if fs::read(&manifest).as_deref().ok() != Some(bytes.as_slice()) {
        fs::write(&manifest, bytes)?;
    }

    Ok(manifest)
}

fn rebase_program_paths<'a>(
    plan: &'a mut Value,
    product: &Path,
    library: &Path,
    sdk_origin: &Path,
    sdk_library: &Path,
) -> Result<&'a mut Vec<Value>, Box<dyn std::error::Error>> {
    let relative_stub = product.join("program__exe.scm");
    let stub = Path::new(
        plan["stub"]
            .as_str()
            .ok_or("compiler program has no stub")?,
    );
    let origin = stub
        .ancestors()
        .nth(relative_stub.components().count())
        .filter(|_| stub.is_absolute() && stub.ends_with(&relative_stub))
        .ok_or("compiler program stub is outside its published directory")?
        .to_path_buf();
    plan["stub"] = serde_json::to_value(library.join(relative_stub))?;
    plan["library_dir"] = serde_json::to_value(sdk_library)?;
    let modules = plan
        .get_mut("modules")
        .and_then(Value::as_array_mut)
        .ok_or("compiler program has no module array")?;
    // Restore package and toolchain inputs against their respective local owners.
    for module in modules.iter_mut() {
        let source = module["scm"]
            .as_str()
            .ok_or("compiler module has no source")?;
        let source = Path::new(source);
        if let Ok(relative) = source.strip_prefix(&origin) {
            module["scm"] = serde_json::to_value(library.join(relative))?;
        } else if module["system"] == true
            && let Ok(relative) = source.strip_prefix(sdk_origin)
        {
            module["scm"] = serde_json::to_value(sdk_library.join(relative))?;
        }
    }
    Ok(modules)
}

/// Marlin receipt emitted after the upstream substrate packages native objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AotArchiveLinkReceipt {
    pub archive_name: String,
    pub archive_file: PathBuf,
    pub cargo_directives: Vec<GerbilAotCargoDirective>,
}

/// Packages the Deck runtime object pair while preserving its public archive name.
pub fn build_agent_policy_routing_static_archive_from_link_plan(
    link_plan: &GerbilAotStaticLinkPlan,
    out_dir: &Path,
) -> Result<AotArchiveLinkReceipt, String> {
    build_static_archive_from_link_plan(AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME, link_plan, out_dir)
}

/// Delegates neutral archive packaging to `gerbil-scheme-aot-build`.
pub fn build_static_archive_from_link_plan(
    archive_name: &str,
    link_plan: &GerbilAotStaticLinkPlan,
    out_dir: &Path,
) -> Result<AotArchiveLinkReceipt, String> {
    let upstream_plan = upstream_link_plan(link_plan);
    let receipt = build_upstream_static_archive(archive_name, &upstream_plan, out_dir)?;
    Ok(AotArchiveLinkReceipt {
        archive_name: receipt.archive_name,
        archive_file: receipt.archive_file,
        cargo_directives: receipt
            .cargo_directives
            .into_iter()
            .map(marlin_cargo_directive)
            .collect(),
    })
}

/// Projects the upstream neutral directives into Marlin's compatibility type.
pub fn static_archive_cargo_directives(
    archive_name: &str,
    archive_dir: &Path,
    link_plan: &GerbilAotStaticLinkPlan,
) -> Vec<GerbilAotCargoDirective> {
    upstream_cargo_directives(archive_name, archive_dir, &upstream_link_plan(link_plan))
        .into_iter()
        .map(marlin_cargo_directive)
        .collect()
}

fn upstream_link_plan(link_plan: &GerbilAotStaticLinkPlan) -> NativeStaticLinkPlan {
    NativeStaticLinkPlan {
        module_objects: link_plan.module_objects.clone(),
        link_object: link_plan.link_object.clone(),
        link_search_dirs: link_plan.link_search_dirs.clone(),
        link_libraries: link_plan
            .link_libraries
            .iter()
            .map(|library| gerbil_scheme_aot_build::NativeLinkLibrary::new(library.as_str()))
            .collect(),
    }
}

fn marlin_cargo_directive(
    directive: gerbil_scheme_aot_build::CargoDirective,
) -> GerbilAotCargoDirective {
    let kind = match directive.kind {
        CargoDirectiveKind::RustcLinkSearch => GerbilAotCargoDirectiveKind::RustcLinkSearch,
        CargoDirectiveKind::RustcLinkLib => GerbilAotCargoDirectiveKind::RustcLinkLib,
    };
    GerbilAotCargoDirective::new(kind, directive.value)
}
