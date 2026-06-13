//! Static archive packaging for generated Gerbil Deck runtime link units.

use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeCargoDirective, GerbilDeckRuntimeNativeCargoDirectiveKind,
    GerbilDeckRuntimeNativeStaticLinkPlan,
};
use std::path::{Path, PathBuf};

/// Cargo/rustc link name for the generated Deck runtime native archive.
pub const DECK_RUNTIME_NATIVE_ARCHIVE_NAME: &str = "marlin_deck_runtime_native";

/// Receipt emitted after the generated Gerbil object pair is packaged as one archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeArchiveLinkReceipt {
    pub archive_name: String,
    pub archive_file: PathBuf,
    pub cargo_directives: Vec<GerbilDeckRuntimeNativeCargoDirective>,
}

/// Packages the `gsc`-produced module and link objects into a static archive.
pub fn build_static_archive_from_link_plan(
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
    out_dir: &Path,
) -> Result<NativeArchiveLinkReceipt, String> {
    std::fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;

    let mut build = cc::Build::new();
    build
        .cargo_metadata(false)
        .out_dir(out_dir)
        .object(&link_plan.module_object)
        .object(&link_plan.link_object);
    build
        .try_compile(DECK_RUNTIME_NATIVE_ARCHIVE_NAME)
        .map_err(|error| error.to_string())?;
    let archive_file = out_dir.join(static_archive_file_name(DECK_RUNTIME_NATIVE_ARCHIVE_NAME));
    if !archive_file.is_file() {
        return Err(format!(
            "static archive was not produced at {}",
            archive_file.display()
        ));
    }

    Ok(NativeArchiveLinkReceipt {
        archive_name: DECK_RUNTIME_NATIVE_ARCHIVE_NAME.to_string(),
        archive_file,
        cargo_directives: static_archive_cargo_directives(
            DECK_RUNTIME_NATIVE_ARCHIVE_NAME,
            out_dir,
            link_plan,
        ),
    })
}

/// Builds Cargo directives for consuming the generated static archive.
pub fn static_archive_cargo_directives(
    archive_name: &str,
    archive_dir: &Path,
    link_plan: &GerbilDeckRuntimeNativeStaticLinkPlan,
) -> Vec<GerbilDeckRuntimeNativeCargoDirective> {
    let mut directives = vec![
        GerbilDeckRuntimeNativeCargoDirective::new(
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkSearch,
            format!("native={}", archive_dir.display()),
        ),
        GerbilDeckRuntimeNativeCargoDirective::new(
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkLib,
            format!("static={archive_name}"),
        ),
    ];

    directives.extend(link_plan.link_search_dirs.iter().map(|search_dir| {
        GerbilDeckRuntimeNativeCargoDirective::new(
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkSearch,
            format!("native={}", search_dir.display()),
        )
    }));
    directives.extend(link_plan.link_libraries.iter().map(|library| {
        GerbilDeckRuntimeNativeCargoDirective::new(
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkLib,
            library.as_str(),
        )
    }));

    directives
}

/// Returns the archive filename produced by `cc` for the current target family.
pub fn static_archive_file_name(archive_name: &str) -> String {
    if cfg!(target_env = "msvc") {
        format!("{archive_name}.lib")
    } else {
        format!("lib{archive_name}.a")
    }
}
