// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rooted Gerbil Scheme C-ABI bridge for Marlin's native `:gerbil-ascent` +
//! `:gerbil-parser` + POO Search DSL (`scheme/search-dsl` + `scheme/c-abi/search-dsl.ss`).

mod choice_codec;
mod codec;
mod render;
mod render_groups;
mod runner;
mod types;

pub use codec::{
    decode_edb_prefilter, decode_rooted_search_dsl_result, decode_search_dsl_completions,
    encode_edb_snapshot, format_rooted_search_dsl_result,
};
pub(crate) use codec::{decode_string, decode_usize, make_string, make_vector, native};
pub use runner::{
    default_marlin_scheme_dir, default_search_dsl_runner, default_search_dsl_session_runner,
    default_search_dsl_structured_runner, execute_linked_search_dsl,
    execute_linked_search_dsl_structured, project_linked_search_dsl_completions,
};
#[cfg(not(feature = "linked-ui"))]
pub use runner::{
    default_package_search_dsl_runner, default_package_search_dsl_session_runner,
    default_package_search_dsl_structured_runner, execute_package_search_dsl,
    execute_package_search_dsl_session_structured, execute_package_search_dsl_structured,
    project_package_search_dsl_completions,
};
pub use types::{
    EdbHydrationMask, OwnedWorkspaceEdbPrefilter, SearchDslChoiceRuleKind, SearchDslChoiceWitness,
    SearchDslChoiceWitnessKind, SearchDslChoiceWorld, SearchDslCompletionEntry,
    SearchDslCompletionKind, SearchDslExecutionMode, SearchDslExecutionRequest,
    SearchDslLinkedExports, SearchDslRunner, SearchDslSessionExecutionRequest, SearchDslSessionId,
    SearchDslSessionRunner, SearchDslStructuredResult, SearchDslStructuredRow,
    SearchDslStructuredRunner, SearchEpistemicStatus, SearchEvidenceChainReceipt,
};
