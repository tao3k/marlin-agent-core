//! Build-time generators for crate-shipped Gerbil runtime assets.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

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
