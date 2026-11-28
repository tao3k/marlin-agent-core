// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed domain protocol records and runners for Marlin's Scheme Search DSL
//! C-ABI bridge (`scheme/search-dsl` + `scheme/c-abi/search-dsl.ss`).

use std::sync::{Arc, RwLock};

use agent_semantic_search::{WorkspaceEdbPrefilter, WorkspaceSearchIndex};

use crate::SearchPlanProjectionError;

pub(crate) const DEFAULT_MAX_EDB_FACTS: usize = 262_144;

/// Typed bitmask governing which optional EDB relation tables are hydrated across
/// the Rust -> Gerbil C-ABI boundary for a query's dependency cone.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct EdbHydrationMask(u8);

impl EdbHydrationMask {
    const EDGES: u8 = 1 << 0;
    const ATTRS: u8 = 1 << 1;
    const SYMBOL_REFS: u8 = 1 << 2;

    /// Default hydration set (`edge` + `attr`, excluding `symbol_ref` unless active).
    pub const DEFAULT_RELATIONS: Self = Self(Self::EDGES | Self::ATTRS);

    /// Build a hydration mask from the active EDB relation symbols returned by the
    /// Scheme prefilter compiler.
    #[must_use]
    pub fn from_active_relations(active_edb: &[String]) -> Self {
        let mut bits = 0u8;
        if active_edb.is_empty() || active_edb.iter().any(|r| r == "edge") {
            bits |= Self::EDGES;
        }
        if active_edb.is_empty() || active_edb.iter().any(|r| r == "attr") {
            bits |= Self::ATTRS;
        }
        if active_edb.iter().any(|r| r == "symbol_ref") {
            bits |= Self::SYMBOL_REFS;
        }
        Self(bits)
    }

    #[must_use]
    pub const fn includes_edges(self) -> bool {
        (self.0 & Self::EDGES) != 0
    }

    #[must_use]
    pub const fn includes_attrs(self) -> bool {
        (self.0 & Self::ATTRS) != 0
    }

    #[must_use]
    pub const fn includes_symbol_refs(self) -> bool {
        (self.0 & Self::SYMBOL_REFS) != 0
    }
}

impl Default for EdbHydrationMask {
    fn default() -> Self {
        Self::DEFAULT_RELATIONS
    }
}

/// Owned storage backing ASP's zero-regex [`WorkspaceEdbPrefilter`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedWorkspaceEdbPrefilter {
    pub languages: Vec<String>,
    pub documents: Vec<String>,
    pub path_prefix: Option<String>,
    pub extension: Option<String>,
    pub required_tokens: Vec<String>,
    pub any_tokens: Vec<String>,
    pub max_files: usize,
    pub max_facts: usize,
    pub hydration: EdbHydrationMask,
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
    Keyword,
    Value,
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
            Self::Keyword => "keyword",
            Self::Value => "value",
        }
    }

    pub(crate) fn parse(raw: &str) -> Result<Self, SearchPlanProjectionError> {
        match raw {
            "ascent" => Ok(Self::Ascent),
            "dusa" => Ok(Self::Dusa),
            "parser" => Ok(Self::Parser),
            "edb" => Ok(Self::Edb),
            "relation" => Ok(Self::Relation),
            "keyword" => Ok(Self::Keyword),
            "value" => Ok(Self::Value),
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

/// Provenance derivation kind emitted by Dusa's finite-choice solver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchDslChoiceWitnessKind {
    Deduction,
    Branch,
    Derived,
}

impl SearchDslChoiceWitnessKind {
    /// Canonical lowercase label for TUI & LLM witness formatting.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deduction => "deduction",
            Self::Branch => "branch",
            Self::Derived => "derived",
        }
    }

    pub(crate) fn parse(raw: &str) -> Self {
        match raw {
            "deduction" => Self::Deduction,
            "branch" => Self::Branch,
            _ => Self::Derived,
        }
    }
}

/// Dusa rule discipline (`closed` deterministic/forbid vs `open` branching).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchDslChoiceRuleKind {
    Closed,
    Open,
    Rule,
}

impl SearchDslChoiceRuleKind {
    /// Canonical lowercase label for TUI & LLM witness formatting.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::Rule => "rule",
        }
    }

    pub(crate) fn parse(raw: &str) -> Self {
        match raw {
            "closed" => Self::Closed,
            "open" => Self::Open,
            _ => Self::Rule,
        }
    }
}

/// One Dusa derivation witness explaining why a choice binding holds in a world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDslChoiceWitness {
    pub kind: SearchDslChoiceWitnessKind,
    pub fact: Vec<String>,
    pub rule_kind: SearchDslChoiceRuleKind,
    pub premises: Vec<String>,
    pub alternatives: Vec<String>,
}

/// One self-consistent world (model) returned by Dusa `search-choice-program`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDslChoiceWorld {
    pub world_index: usize,
    pub bindings: Vec<Vec<String>>,
    pub witnesses: Vec<SearchDslChoiceWitness>,
}

/// Validated semantic identifier binding multi-turn L1/L2 search queries to a
/// persistent Scheme `evidence-chain.ss` truth-maintenance session.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SearchDslSessionId(String);

impl SearchDslSessionId {
    /// Validate and construct a non-empty session identifier (at most 128 bytes).
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let value = raw.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.len() > 128 {
            return Err(SearchPlanProjectionError::Invalid(
                "search DSL session id must be 1..=128 bytes",
            ));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// Borrow the canonical session identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Epistemic convergence state emitted by Scheme `evidence-chain.ss`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum SearchEpistemicStatus {
    #[default]
    Exploring,
    CompetingWorlds,
    UniqueDeduction,
    ConvergedEvidence,
    EpistemicConflict,
}

impl SearchEpistemicStatus {
    /// Canonical lowercase kebab-case identifier shared with gRPC & telemetry.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exploring => "exploring",
            Self::CompetingWorlds => "competing-worlds",
            Self::UniqueDeduction => "unique-deduction",
            Self::ConvergedEvidence => "converged-evidence",
            Self::EpistemicConflict => "epistemic-conflict",
        }
    }

    pub(crate) fn parse(raw: &str) -> Result<Self, SearchPlanProjectionError> {
        match raw {
            "exploring" => Ok(Self::Exploring),
            "competing-worlds" => Ok(Self::CompetingWorlds),
            "unique-deduction" => Ok(Self::UniqueDeduction),
            "converged-evidence" => Ok(Self::ConvergedEvidence),
            "epistemic-conflict" => Ok(Self::EpistemicConflict),
            _ => Err(SearchPlanProjectionError::Invalid(
                "unknown search epistemic status",
            )),
        }
    }
}

/// Typed evidence-chain receipt projected from Scheme `evidence-chain.ss`
/// (`#(evidence-receipt epoch total-m1 new-m1 ambiguous conflicts covered open status)`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchEvidenceChainReceipt {
    pub epoch: usize,
    pub total_m1_claims: usize,
    pub newly_harvested: usize,
    pub ambiguous_branches: Vec<String>,
    pub conflict_count: usize,
    pub covered_layers: Vec<String>,
    pub open_frontiers: Vec<String>,
    pub epistemic_status: SearchEpistemicStatus,
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
    pub canonical_query: Option<String>,
    pub rows: Vec<SearchDslStructuredRow>,
    pub choice_worlds: Vec<SearchDslChoiceWorld>,
    pub budget_exhausted: Option<String>,
    pub evidence_receipt: Option<SearchEvidenceChainReceipt>,
    pub rendered_text: String,
}

/// Optional host context passed to the rooted Search DSL decoder so rendered
/// output reports accurate scoped vs workspace coverage and attaches zero-I/O
/// RAM source snippets directly to L1 Ascent answer rows.
#[derive(Clone, Copy)]
pub(crate) struct SearchDslDecodeContext<'a> {
    pub(crate) index: Option<&'a WorkspaceSearchIndex>,
    pub(crate) prefilter: Option<&'a OwnedWorkspaceEdbPrefilter>,
    pub(crate) output_limit: usize,
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

/// Typed session-scoped search execution request enabling live M1 evidence
/// harvesting and cross-turn Open EDB amplification via `evidence-chain.ss`.
#[derive(Clone, Copy)]
pub struct SearchDslSessionExecutionRequest<'a> {
    pub execution: SearchDslExecutionRequest<'a>,
    pub session_id: &'a SearchDslSessionId,
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

/// Host-supplied session-aware runner executing native `:gerbil-ascent` / Dusa
/// queries with live `evidence-chain.ss` M1 harvesting and typed receipt projection.
pub type SearchDslSessionRunner = Arc<
    dyn Fn(
            Arc<RwLock<WorkspaceSearchIndex>>,
            &str,
            usize,
            &SearchDslSessionId,
        ) -> Result<SearchDslStructuredResult, SearchPlanProjectionError>
        + Send
        + Sync,
>;
