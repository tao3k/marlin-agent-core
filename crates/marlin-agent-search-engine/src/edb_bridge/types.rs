// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed domain protocol records and runners for Marlin's Scheme Search DSL
//! C-ABI bridge (`scheme/search-dsl` + `scheme/c-abi/search-dsl.ss`).

use std::sync::{Arc, RwLock};

use agent_semantic_search::{WorkspaceEdbPrefilter, WorkspaceSearchIndex};

use crate::SearchPlanProjectionError;

/// Owned storage backing ASP's zero-regex [`WorkspaceEdbPrefilter`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedWorkspaceEdbPrefilter {
    pub languages: Vec<String>,
    pub documents: Vec<String>,
    pub path_prefix: Option<String>,
    pub extension: Option<String>,
    pub required_tokens: Vec<String>,
    pub max_files: usize,
    pub max_facts: usize,
}

impl OwnedWorkspaceEdbPrefilter {
    /// Borrow as ASP's [`WorkspaceEdbPrefilter`].
    #[must_use]
    pub fn as_prefilter(&self) -> WorkspaceEdbPrefilter<'_> {
        WorkspaceEdbPrefilter {
            languages: &self.languages,
            documents: &self.documents,
            path_prefix: self.path_prefix.as_deref(),
            extension: self.extension.as_deref(),
            required_tokens: &self.required_tokens,
            max_files: self.max_files,
            max_facts: self.max_facts,
        }
    }
}

/// Semantic category of an interactive Helm TAB completion row.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchDslCompletionKind {
    Ascent,
    Dusa,
    Parser,
    Edb,
    Relation,
}

impl SearchDslCompletionKind {
    /// Canonical lowercase label for UI columns and protocol matching.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ascent => "ascent",
            Self::Dusa => "dusa",
            Self::Parser => "parser",
            Self::Edb => "edb",
            Self::Relation => "relation",
        }
    }

    pub(crate) fn parse(raw: &str) -> Result<Self, SearchPlanProjectionError> {
        match raw {
            "ascent" => Ok(Self::Ascent),
            "dusa" => Ok(Self::Dusa),
            "parser" => Ok(Self::Parser),
            "edb" => Ok(Self::Edb),
            "relation" => Ok(Self::Relation),
            _ => Err(SearchPlanProjectionError::Invalid(
                "unknown search DSL completion kind",
            )),
        }
    }
}

/// Execution backend mode reported by `scheme/search-dsl`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchDslExecutionMode {
    Ascent,
    Choice,
    GerbilParser,
}

impl SearchDslExecutionMode {
    /// Canonical lowercase mode identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ascent => "ascent",
            Self::Choice => "choice",
            Self::GerbilParser => "gerbil-parser",
        }
    }

    pub(crate) fn parse(raw: &str) -> Result<Self, SearchPlanProjectionError> {
        match raw {
            "ascent" => Ok(Self::Ascent),
            "choice" => Ok(Self::Choice),
            "gerbil-parser" => Ok(Self::GerbilParser),
            _ => Err(SearchPlanProjectionError::Invalid(
                "unknown search DSL execution mode",
            )),
        }
    }
}

/// One structured answer row projected from `:gerbil-ascent` / Dusa / `:gerbil-parser`
/// for direct Helm navigation, RAM preview, and Embark actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDslStructuredRow {
    pub cells: Vec<String>,
    pub display: String,
    pub path: Option<String>,
    pub line: Option<usize>,
}

/// Complete typed result of a Scheme Search DSL execution shared by both
/// LLM `workspace_search` tool calls and TUI Helm live S-expression evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDslStructuredResult {
    pub generation_digest: String,
    pub elapsed_micros: u64,
    pub indexed_files: usize,
    pub matched_files: usize,
    pub mode: SearchDslExecutionMode,
    pub relation: String,
    pub repairs: Vec<String>,
    pub rows: Vec<SearchDslStructuredRow>,
    pub rendered_text: String,
}

/// One interactive Helm TAB completion row introspected from Scheme's POO
/// `SearchLanguageRegistry.` and real `:gerbil-ascent` relation declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDslCompletionEntry {
    pub form: String,
    pub kind: SearchDslCompletionKind,
    pub signature: String,
    pub language: String,
    pub doc: String,
    pub insert_text: String,
}

/// Typed request executing a native `:gerbil-ascent` query against a resident
/// [`WorkspaceSearchIndex`].
#[derive(Clone, Copy)]
pub struct SearchDslExecutionRequest<'a> {
    pub index: &'a WorkspaceSearchIndex,
    pub query: &'a str,
    pub output_limit: usize,
}

/// AOT-linked C-ABI exports from `scheme/c-abi/search-dsl.ss`.
#[derive(Clone, Copy)]
pub struct SearchDslLinkedExports {
    pub compile_prefilter: unsafe extern "C" fn(i64) -> i64,
    pub eval_edb: unsafe extern "C" fn(i64) -> i64,
    pub extract_facts: unsafe extern "C" fn(i64) -> i64,
    pub completions: unsafe extern "C" fn(i64) -> i64,
    pub register_provider_descriptor: unsafe extern "C" fn(i64) -> i64,
}

/// Host-supplied runner executing a native `:gerbil-ascent` query against a
/// shared resident [`WorkspaceSearchIndex`] and returning formatted evidence.
pub type SearchDslRunner = Arc<
    dyn Fn(
            Arc<RwLock<WorkspaceSearchIndex>>,
            &str,
            usize,
        ) -> Result<String, SearchPlanProjectionError>
        + Send
        + Sync,
>;

/// Host-supplied runner returning structured [`SearchDslStructuredResult`] rows
/// for interactive Helm navigation, preview, and Embark dispatch.
pub type SearchDslStructuredRunner = Arc<
    dyn Fn(
            Arc<RwLock<WorkspaceSearchIndex>>,
            &str,
            usize,
        ) -> Result<SearchDslStructuredResult, SearchPlanProjectionError>
        + Send
        + Sync,
>;
