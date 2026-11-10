//! Marlin archive adapters over the upstream Gerbil native build substrate.

use gerbil_scheme_aot_build::{
    CargoDirectiveKind, NativeStaticLinkPlan,
    build_static_archive_from_link_plan as build_upstream_static_archive,
    static_archive_cargo_directives as upstream_cargo_directives,
};
use marlin_gerbil_scheme::{
    GerbilDeckRouteAbiCargoDirective, GerbilDeckRouteAbiCargoDirectiveKind,
    GerbilDeckRouteAbiStaticLinkPlan,
};
use std::path::{Path, PathBuf};

pub use gerbil_scheme_aot_build::static_archive_file_name;

/// Cargo/rustc link name for the generated Deck runtime native archive.
pub const DECK_ROUTE_ABI_ARCHIVE_NAME: &str = "marlin_deck_route_abi";
/// Cargo/rustc link name for the generated AgentGraph policy-routing native archive.
pub const AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME: &str = "marlin_agent_policy_routing_abi";

/// Marlin receipt emitted after the upstream substrate packages native objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AotArchiveLinkReceipt {
    pub archive_name: String,
    pub archive_file: PathBuf,
    pub cargo_directives: Vec<GerbilDeckRouteAbiCargoDirective>,
}

/// Packages the Deck runtime object pair while preserving its public archive name.
pub fn build_deck_runtime_static_archive_from_link_plan(
    link_plan: &GerbilDeckRouteAbiStaticLinkPlan,
    out_dir: &Path,
) -> Result<AotArchiveLinkReceipt, String> {
    build_static_archive_from_link_plan(DECK_ROUTE_ABI_ARCHIVE_NAME, link_plan, out_dir)
}

/// Packages the policy-routing object pair while preserving its public archive name.
pub fn build_agent_policy_routing_static_archive_from_link_plan(
    link_plan: &GerbilDeckRouteAbiStaticLinkPlan,
    out_dir: &Path,
) -> Result<AotArchiveLinkReceipt, String> {
    build_static_archive_from_link_plan(AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME, link_plan, out_dir)
}

/// Delegates neutral archive packaging to `gerbil-scheme-aot-build`.
pub fn build_static_archive_from_link_plan(
    archive_name: &str,
    link_plan: &GerbilDeckRouteAbiStaticLinkPlan,
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
    link_plan: &GerbilDeckRouteAbiStaticLinkPlan,
) -> Vec<GerbilDeckRouteAbiCargoDirective> {
    upstream_cargo_directives(archive_name, archive_dir, &upstream_link_plan(link_plan))
        .into_iter()
        .map(marlin_cargo_directive)
        .collect()
}

fn upstream_link_plan(link_plan: &GerbilDeckRouteAbiStaticLinkPlan) -> NativeStaticLinkPlan {
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
) -> GerbilDeckRouteAbiCargoDirective {
    let kind = match directive.kind {
        CargoDirectiveKind::RustcLinkSearch => {
            GerbilDeckRouteAbiCargoDirectiveKind::RustcLinkSearch
        }
        CargoDirectiveKind::RustcLinkLib => GerbilDeckRouteAbiCargoDirectiveKind::RustcLinkLib,
    };
    GerbilDeckRouteAbiCargoDirective::new(kind, directive.value)
}
