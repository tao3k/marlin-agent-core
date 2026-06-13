//! Build-script support for the native Deck runtime integration crate.

mod archive;
mod discovery;
mod header;
mod script;
mod toolchain;

pub use archive::{
    DECK_RUNTIME_NATIVE_ARCHIVE_NAME, NativeArchiveLinkReceipt,
    build_static_archive_from_link_plan, static_archive_cargo_directives, static_archive_file_name,
};
pub use header::{NativeCHeaderGenerationReceipt, write_native_c_header};
pub use script::{emit_deck_runtime_native_link_directives, rustc_wrapper_is_clippy};
pub use toolchain::{NativeCCompilerTool, discover_native_c_compiler};
