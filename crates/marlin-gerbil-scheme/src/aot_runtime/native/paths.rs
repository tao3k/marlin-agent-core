//! Path derivation for native Gerbil Deck runtime AOT artifacts.

use crate::runtime::GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH;
use std::path::{Path, PathBuf};

pub(super) const GERBIL_DECK_RUNTIME_NATIVE_AOT_OUTPUT_DIR: &str = ".gerbil/native";
pub(super) const GERBIL_DECK_RUNTIME_NATIVE_PACKAGE: &str = "marlin-deck-runtime";
pub(super) const GERBIL_DECK_RUNTIME_NATIVE_SELECT_SYMBOL: &str =
    "marlin_deck_runtime_select_model_route";
pub(super) const GERBIL_DECK_RUNTIME_NATIVE_INITIALIZE_SYMBOL: &str =
    "marlin_deck_runtime_initialize";

pub(super) fn native_output_dir(root: &Path) -> PathBuf {
    root.join(GERBIL_DECK_RUNTIME_NATIVE_AOT_OUTPUT_DIR)
}

pub(super) fn native_scheme_source(root: &Path) -> PathBuf {
    root.join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH)
}

pub(super) fn generated_module_stem(output_dir: &Path) -> PathBuf {
    output_dir
        .join(GERBIL_DECK_RUNTIME_NATIVE_PACKAGE)
        .join("src/marlin/deck-runtime-native")
}

pub(super) fn generated_loader_scm(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_extension("scm")
}

pub(super) fn generated_runtime_scm(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_file_name("deck-runtime-native~0.scm")
}

pub(super) fn generated_ssi(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_extension("ssi")
}

pub(super) fn generated_ssxi(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_extension("ssxi.ss")
}

pub(super) fn static_scm(output_dir: &Path) -> PathBuf {
    output_dir
        .join("static")
        .join("marlin-deck-runtime__src__marlin__deck-runtime-native.scm")
}

pub(super) fn generated_object(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_file_name("deck-runtime-native~0.o")
}

pub(super) fn generated_link_c_source(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_file_name("deck-runtime-native~0_.c")
}

pub(super) fn generated_link_object(output_dir: &Path) -> PathBuf {
    generated_module_stem(output_dir).with_file_name("deck-runtime-native~0_.o")
}
