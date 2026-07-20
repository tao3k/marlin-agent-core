//! Marlin archive adapters over the upstream Gerbil native build substrate.

use gerbil_scheme_native_build::{
    CargoDirectiveKind, NativeStaticLinkPlan,
    build_static_archive_from_link_plan as build_upstream_static_archive,
    static_archive_cargo_directives as upstream_cargo_directives,
};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeCargoDirective, GerbilDeckRuntimeNativeCargoDirectiveKind,
    GerbilDeckRuntimeNativeStaticLinkPlan,
};
use std::path::{Path, PathBuf};

pub use gerbil_scheme_native_build::static_archive_file_name;

/// Cargo/rustc link name for the generated Deck runtime native archive.
pub const DECK_RUNTIME_NATIVE_ARCHIVE_NAME: &str = "marlin_deck_runtime_native";
/// Cargo/rustc link name for the generated AgentGraph policy-routing native archive.
pub const AGENT_POLICY_ROUTING_NATIVE_ARCHIVE_NAME: &str = "marlin_agent_policy_routing_native";

/// Marlin receipt emitted after the upstream substrate packages native objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeArchiveLinkReceipt {
    pub archive_name: String,
    pub archive_file: PathBuf,
    pub cargo_directives: Vec<GerbilDeckRuntimeNativeCargoDirective>,
}

/// Packages the Deck runtime object pair while preserving its public archive name.
pub fn build_deck_runtime_static_archive_from_link_plan(
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
    out_dir: &Path,
) -> Result<NativeArchiveLinkReceipt, String> {
    build_static_archive_from_link_plan(DECK_RUNTIME_NATIVE_ARCHIVE_NAME, link_plan, out_dir)
}

/// Packages the policy-routing object pair while preserving its public archive name.
pub fn build_agent_policy_routing_static_archive_from_link_plan(
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
    out_dir: &Path,
) -> Result<NativeArchiveLinkReceipt, String> {
    build_static_archive_from_link_plan(
        AGENT_POLICY_ROUTING_NATIVE_ARCHIVE_NAME,
        link_plan,
        out_dir,
    )
}

/// Delegates neutral archive packaging to `gerbil-scheme-native-build`.
pub fn build_static_archive_from_link_plan(
    archive_name: &str,
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
    out_dir: &Path,
) -> Result<NativeArchiveLinkReceipt, String> {
    let upstream_plan = upstream_link_plan(link_plan);
    let receipt = build_upstream_static_archive(archive_name, &upstream_plan, out_dir)?;
    Ok(NativeArchiveLinkReceipt {
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
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
) -> Vec<GerbilDeckRuntimeNativeCargoDirective> {
    upstream_cargo_directives(archive_name, archive_dir, &upstream_link_plan(link_plan))
        .into_iter()
        .map(marlin_cargo_directive)
        .collect()
}

fn upstream_link_plan(link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan) -> NativeStaticLinkPlan {
    NativeStaticLinkPlan {
        module_objects: link_plan.module_objects.clone(),
        link_object: link_plan.link_object.clone(),
        link_search_dirs: link_plan.link_search_dirs.clone(),
        link_libraries: link_plan
            .link_libraries
            .iter()
            .map(|library| gerbil_scheme_native_build::NativeLinkLibrary::new(library.as_str()))
            .collect(),
    }
}

fn marlin_cargo_directive(
    directive: gerbil_scheme_native_build::CargoDirective,
) -> GerbilDeckRuntimeNativeCargoDirective {
    let kind = match directive.kind {
        CargoDirectiveKind::RustcLinkSearch => {
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkSearch
        }
        CargoDirectiveKind::RustcLinkLib => GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkLib,
    };
    GerbilDeckRuntimeNativeCargoDirective::new(kind, directive.value)
}
