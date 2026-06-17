//! Build-script support for Gerbil native integration crates.

mod archive;
mod discovery;
mod header;
mod script;
mod toolchain;

pub use archive::{
    AGENT_POLICY_ROUTING_NATIVE_ARCHIVE_NAME, DECK_RUNTIME_NATIVE_ARCHIVE_NAME,
    NativeArchiveLinkReceipt, build_agent_policy_routing_static_archive_from_link_plan,
    build_deck_runtime_static_archive_from_link_plan, build_static_archive_from_link_plan,
    static_archive_cargo_directives, static_archive_file_name,
};
pub use header::{NativeCHeaderGenerationReceipt, write_native_c_header};
pub use script::{
    emit_agent_policy_routing_native_link_directives, emit_deck_runtime_native_link_directives,
    rustc_wrapper_is_clippy,
};
pub use toolchain::{NativeCCompilerTool, discover_native_c_compiler};
