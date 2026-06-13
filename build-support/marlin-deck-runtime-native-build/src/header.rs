//! C header generation helpers for native Deck runtime ABI audits.

use std::path::{Path, PathBuf};

/// Receipt emitted after `cbindgen` writes a C header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCHeaderGenerationReceipt {
    pub crate_dir: PathBuf,
    pub header_file: PathBuf,
}

/// Generates a C header from a Rust crate using `cbindgen`.
pub fn write_native_c_header(
    crate_dir: &Path,
    header_file: &Path,
    include_guard: &str,
) -> Result<NativeCHeaderGenerationReceipt, String> {
    if let Some(parent) = header_file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        include_guard: Some(include_guard.to_string()),
        ..Default::default()
    };

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .map_err(|error| error.to_string())?
        .write_to_file(header_file);
    if !header_file.is_file() {
        return Err(format!(
            "native C header was not produced at {}",
            header_file.display()
        ));
    }

    Ok(NativeCHeaderGenerationReceipt {
        crate_dir: crate_dir.to_path_buf(),
        header_file: header_file.to_path_buf(),
    })
}
