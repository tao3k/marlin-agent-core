//! Path derivation for native Gerbil Deck runtime AOT artifacts.

use std::path::{Path, PathBuf};

pub(super) const GERBIL_DECK_RUNTIME_NATIVE_AOT_OUTPUT_DIR: &str = ".gerbil/native";
const GERBIL_DECK_RUNTIME_NATIVE_ARTIFACT_STEM: &str = "deck-runtime-native~0";
pub(super) const GERBIL_DECK_RUNTIME_NATIVE_SELECT_SYMBOL: &str =
    "marlin_deck_runtime_select_model_route";
pub(super) const GERBIL_DECK_RUNTIME_NATIVE_INITIALIZE_SYMBOL: &str =
    "marlin_deck_runtime_initialize";

pub(super) fn native_output_dir(root: &Path) -> PathBuf {
    root.join(GERBIL_DECK_RUNTIME_NATIVE_AOT_OUTPUT_DIR)
}

pub(super) fn default_compiled_runtime_scm(output_dir: &Path) -> PathBuf {
    output_dir.join(format!("{GERBIL_DECK_RUNTIME_NATIVE_ARTIFACT_STEM}.scm"))
}

pub(super) fn compiled_runtime_object(compiled_runtime_scm: &Path) -> PathBuf {
    compiled_runtime_artifact_path(compiled_runtime_scm, "", "o")
}

pub(super) fn compiled_runtime_link_c_source(compiled_runtime_scm: &Path) -> PathBuf {
    compiled_runtime_artifact_path(compiled_runtime_scm, "_", "c")
}

pub(super) fn compiled_runtime_link_object(compiled_runtime_scm: &Path) -> PathBuf {
    compiled_runtime_artifact_path(compiled_runtime_scm, "_", "o")
}

fn compiled_runtime_artifact_path(
    compiled_runtime_scm: &Path,
    stem_suffix: &str,
    extension: &str,
) -> PathBuf {
    let stem = compiled_runtime_scm
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(GERBIL_DECK_RUNTIME_NATIVE_ARTIFACT_STEM);
    compiled_runtime_scm.with_file_name(format!("{stem}{stem_suffix}.{extension}"))
}
