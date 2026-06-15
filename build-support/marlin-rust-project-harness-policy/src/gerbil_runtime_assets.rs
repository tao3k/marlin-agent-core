//! Build-time generators for crate-shipped Gerbil runtime assets.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;

const REQUIRED_GERBIL_RUNTIME_ASSETS: &[&str] = &[
    "gerbil.pkg",
    "build.ss",
    "src/marlin/protocol-types.ss",
    "src/marlin/protocol.ss",
    "src/marlin/request.ss",
    "src/marlin/adapter.ss",
    "src/marlin/deck-runtime.ss",
    "src/marlin/deck-runtime-native.ss",
    "src/marlin/deck-runtime-native-projection.ss",
    "src/marlin/deck-runtime-script.ss",
    "src/marlin/deck-runtime-strategy.ss",
];

/// Build-gate status for crate-shipped Gerbil runtime assets.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum GerbilRuntimeAssetManifestStatus {
    /// The crate has no `gerbil/` runtime root.
    NotPresent,
    /// The `gerbil/` runtime root contains all required Marlin runtime assets.
    Complete,
    /// The `gerbil/` runtime root exists but misses required Marlin assets.
    MissingRequiredAssets,
}

/// Build-gate receipt for crate-shipped Gerbil runtime assets.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GerbilRuntimeAssetManifestReceipt {
    /// Cargo package root inspected by the build gate.
    pub project_root: PathBuf,
    /// Expected Gerbil runtime root under the package.
    pub gerbil_root: PathBuf,
    /// Build-gate status for this package.
    pub status: GerbilRuntimeAssetManifestStatus,
    /// Number of emitted runtime asset files.
    pub asset_count: usize,
    /// Number of `.ss` runtime source files.
    pub scheme_source_count: usize,
    /// Number of `gerbil.pkg` package manifest files.
    pub package_manifest_count: usize,
    /// Required Marlin asset paths checked when the runtime root exists.
    pub required_assets: Vec<String>,
    /// Missing required Marlin asset paths.
    pub missing_required_assets: Vec<String>,
    /// Runtime asset paths relative to the `gerbil/` root.
    pub asset_paths: Vec<String>,
}

impl GerbilRuntimeAssetManifestReceipt {
    /// Returns true when this receipt does not block the build gate.
    pub fn is_success(&self) -> bool {
        !matches!(
            self.status,
            GerbilRuntimeAssetManifestStatus::MissingRequiredAssets
        )
    }

    /// Returns true when the package owns a Gerbil runtime root.
    pub fn has_runtime_assets(&self) -> bool {
        !matches!(self.status, GerbilRuntimeAssetManifestStatus::NotPresent)
    }
}

/// Inspects crate-shipped Gerbil runtime assets under `gerbil/`.
pub fn inspect_gerbil_runtime_assets(project_root: &Path) -> GerbilRuntimeAssetManifestReceipt {
    let gerbil_root = project_root.join("gerbil");
    let required_assets = REQUIRED_GERBIL_RUNTIME_ASSETS
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();

    if !gerbil_root.is_dir() {
        return GerbilRuntimeAssetManifestReceipt {
            project_root: project_root.to_path_buf(),
            gerbil_root,
            status: GerbilRuntimeAssetManifestStatus::NotPresent,
            asset_count: 0,
            scheme_source_count: 0,
            package_manifest_count: 0,
            required_assets,
            missing_required_assets: Vec::new(),
            asset_paths: Vec::new(),
        };
    }

    let mut asset_paths = Vec::new();
    collect_gerbil_runtime_asset_paths(&gerbil_root, &gerbil_root, &mut asset_paths)
        .expect("collect Gerbil runtime asset paths for build gate");
    asset_paths.sort();

    let missing_required_assets = required_assets
        .iter()
        .filter(|required| !asset_paths.iter().any(|asset| asset == *required))
        .cloned()
        .collect::<Vec<_>>();
    let status = if missing_required_assets.is_empty() {
        GerbilRuntimeAssetManifestStatus::Complete
    } else {
        GerbilRuntimeAssetManifestStatus::MissingRequiredAssets
    };
    let scheme_source_count = asset_paths
        .iter()
        .filter(|asset| asset.ends_with(".ss"))
        .count();
    let package_manifest_count = asset_paths
        .iter()
        .filter(|asset| asset.as_str() == "gerbil.pkg")
        .count();

    GerbilRuntimeAssetManifestReceipt {
        project_root: project_root.to_path_buf(),
        gerbil_root,
        status,
        asset_count: asset_paths.len(),
        scheme_source_count,
        package_manifest_count,
        required_assets,
        missing_required_assets,
        asset_paths,
    }
}

/// Generates a Rust manifest for Gerbil runtime assets under `gerbil/`.
pub fn generate_gerbil_runtime_assets(project_root: &Path) {
    let gerbil_root = project_root.join("gerbil");
    let mut paths = Vec::new();
    collect_gerbil_runtime_asset_paths(&gerbil_root, &gerbil_root, &mut paths)
        .expect("collect Gerbil runtime asset paths");
    paths.sort();

    let mut output = String::from(
        "/// Complete file manifest required under a `GERBIL_LOADPATH` root.\n\
         pub const GERBIL_RUNTIME_ASSETS: &[GerbilRuntimeAsset] = &[\n",
    );
    for path in &paths {
        println!(
            "cargo:rerun-if-changed={}",
            gerbil_root.join(path).display()
        );
        output.push_str("    GerbilRuntimeAsset {\n");
        output.push_str("        path: \"");
        output.push_str(&escape_rust_string(path));
        output.push_str("\",\n");
        output.push_str(
            "        source: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/gerbil/",
        );
        output.push_str(&escape_rust_string(path));
        output.push_str("\")),\n");
        output.push_str("    },\n");
    }
    output.push_str("];\n");

    fs::write(out_dir().join("gerbil_runtime_assets.rs"), output)
        .expect("write generated Gerbil runtime asset manifest");
    println!("cargo:rerun-if-changed={}", gerbil_root.display());
}

fn collect_gerbil_runtime_asset_paths(
    root: &Path,
    dir: &Path,
    paths: &mut Vec<String>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if is_transient_gerbil_runtime_asset_dir(&path) {
                continue;
            }
            collect_gerbil_runtime_asset_paths(root, &path, paths)?;
            continue;
        }
        if !is_gerbil_runtime_asset(&path) {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("runtime asset path under Gerbil root")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        paths.push(relative);
    }

    Ok(())
}

fn is_transient_gerbil_runtime_asset_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".cache" | ".devenv" | ".direnv" | ".git" | ".run"))
}

fn is_gerbil_runtime_asset(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "gerbil.pkg")
        || path.extension().is_some_and(|extension| extension == "ss")
}

fn escape_rust_string(value: &str) -> String {
    value
        .chars()
        .flat_map(char::escape_default)
        .collect::<String>()
}

fn out_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR set by Cargo for build scripts"))
}
