// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Build-script support for Gerbil native integration crates.

mod archive;
mod deck_actor;
mod discovery;
mod header;
mod script;
mod toolchain;

pub use archive::{
    AGENT_POLICY_ROUTING_ABI_ARCHIVE_NAME, AotArchiveLinkReceipt,
    build_agent_policy_routing_static_archive_from_link_plan, build_static_archive_from_link_plan,
    static_archive_cargo_directives, static_archive_file_name,
};
pub use deck_actor::emit_deck_actor_link_directives;
pub use header::{CHeaderGenerationReceipt, write_c_header};
pub use script::{emit_agent_policy_routing_abi_link_directives, rustc_wrapper_is_clippy};
pub use toolchain::{CCompilerTool, discover_c_compiler};
