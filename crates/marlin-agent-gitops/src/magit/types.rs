// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed domain protocol for Marlin's AI-Native Magit engine
//! (`jj` + Git + Lambda Aitia gRPC + Human-AI Joint Evidence Chain).

use serde::{Deserialize, Serialize};

/// Active repository backend detected for a workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MagitBackendKind {
    /// Colocated Jujutsu + Git repository (`.jj` + `.git`).
    JjColocated,
    /// Native Git repository (`.git`).
    GitNative,
}

impl MagitBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JjColocated => "jj-colocated",
            Self::GitNative => "git",
        }
    }
}

/// Stable Jujutsu Change-ID (e.g. `kntqzsqt`) surviving amends and rebases.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MagitChangeId(String);

impl MagitChangeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Abbreviated or full Git commit SHA.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MagitCommitSha(String);

impl MagitCommitSha {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Jujutsu Operation Log ID for reversible time-travel undo/restore.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MagitOpId(String);

impl MagitOpId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Evidence-grounded audit risk tier assigned to a hunk or file.
/// Never fabricated from blind regex — elevated only by real Lambda Aitia
/// findings, unresolved conflicts, pending formal models, or Human/LLM flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MagitAuditRisk {
    Info,
    Low,
    Unassessed,
    Medium,
    High,
}

impl MagitAuditRisk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Unassessed => "unassessed",
            Self::Low => "low",
            Self::Info => "info",
        }
    }
}

/// Structural surface concern cluster (path-based grouping for human review).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MagitAuditConcern {
    SecurityBoundary,
    ProtocolAbi,
    ConcurrencyState,
    CoreLogic,
    UiLayout,
    ConfigBuild,
    TestsDocs,
}

impl MagitAuditConcern {
    pub fn label(self) -> &'static str {
        match self {
            Self::SecurityBoundary => "security-boundary",
            Self::ProtocolAbi => "protocol-abi",
            Self::ConcurrencyState => "concurrency-state",
            Self::CoreLogic => "core-logic",
            Self::UiLayout => "ui-layout",
            Self::ConfigBuild => "config-build",
            Self::TestsDocs => "tests-docs",
        }
    }
}

/// Explicit provenance source for one item in a hunk's audit evidence chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MagitEvidenceSource {
    AitiaGrpc,
    FormalModel,
    HumanAudit,
    LlmAudit,
    Structural,
}

impl MagitEvidenceSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AitiaGrpc => "aitia-grpc",
            Self::FormalModel => "formal-model",
            Self::HumanAudit => "human-audit",
            Self::LlmAudit => "llm-audit",
            Self::Structural => "structural",
        }
    }
}

/// Formal verification artifact status for a touched file/crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MagitFormalStatus {
    Verified,
    ModelPresentUnrun,
    None,
}

impl MagitFormalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::ModelPresentUnrun => "model-present-unrun",
            Self::None => "none",
        }
    }
}

/// One cited evidence record in a hunk's Human + LLM + Aitia + Formal evidence chain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitEvidenceItem {
    pub source: MagitEvidenceSource,
    pub verdict: String,       // e.g. "finding", "admit", "audited", "flagged", "model-present"
    pub rule_or_actor: String, // e.g. "g2/silent-fallback", "human", "llm+human", "dev/quint/..."
    pub summary: String,
    pub line: Option<u32>,
}

/// Persisted Human / LLM joint audit verdict attached to a specific hunk.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitJointAuditRecord {
    pub hunk_id: String,
    pub actor: String,   // "human" | "llm" | "joint"
    pub verdict: String, // "audited" | "flagged" | "disputed" | "unreviewed"
    pub rationale: String,
    pub context_digest: Option<String>,
}

/// Magit status section ownership for a file entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MagitFileSection {
    Conflicts,
    Unstaged,
    Staged,
    Untracked,
}

impl MagitFileSection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged => "unstaged",
            Self::Staged => "staged",
            Self::Untracked => "untracked",
        }
    }
}

/// Classification of one line within a unified diff hunk.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MagitDiffLineKind {
    Context,
    Added,
    Removed,
}

/// Single line inside a parsed unified diff hunk.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitDiffLine {
    pub kind: MagitDiffLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub text: String,
}

/// One structured unified diff hunk with cited multi-source evidence chain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitDiffHunk {
    pub hunk_id: String,
    pub file_path: String,
    pub header: String,
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub added: u32,
    pub removed: u32,
    pub risk: MagitAuditRisk,
    pub concern: MagitAuditConcern,
    pub formal_status: MagitFormalStatus,
    pub risk_reason: String,
    pub evidence: Vec<MagitEvidenceItem>,
    pub lines: Vec<MagitDiffLine>,
}

/// One changed file entry inside a Magit status section.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitFileEntry {
    pub path: String,
    pub original_path: Option<String>,
    pub section: MagitFileSection,
    pub status_code: String,
    pub added: u32,
    pub removed: u32,
    pub risk: MagitAuditRisk,
    pub concern: MagitAuditConcern,
    pub risk_reason: String,
    pub hunks: Vec<MagitDiffHunk>,
}

/// One recent commit / `jj` change entry in the Magit log section.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitLogEntry {
    pub change_id: Option<MagitChangeId>,
    pub commit_sha: MagitCommitSha,
    pub bookmarks: Vec<String>,
    pub summary: String,
    pub author: String,
    pub relative_time: String,
    pub has_conflict: bool,
    pub is_working_copy: bool,
}

/// One Git stash entry (`magit-insert-stashes`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitStashEntry {
    pub reference: String,
    pub summary: String,
    pub relative_time: String,
}

/// One Jujutsu operation log entry for safe reversible time-travel undo.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitOpLogEntry {
    pub op_id: MagitOpId,
    pub description: String,
    pub timestamp: String,
    pub is_current: bool,
}

/// Lambda Aitia causal provenance + real gRPC Defense Audit report bound to a snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitAitiaProvenance {
    pub evidence_id: String,
    pub repo_root: String,
    pub backend: MagitBackendKind,
    pub head_commit: Option<MagitCommitSha>,
    pub change_id: Option<MagitChangeId>,
    pub op_id: Option<MagitOpId>,
    pub aitia_online: bool,
    pub aitia_decision: String,
    pub aitia_engine_owner: String,
    pub aitia_report_digest: Option<String>,
    pub causal_acyclic: Option<bool>,
    pub aitia_total_findings: u32,
    pub aitia_note: String,
    pub high_risk_hunks: u32,
    pub medium_risk_hunks: u32,
    pub total_hunks: u32,
    pub conflict_count: u32,
}

/// Complete authoritative Magit status snapshot projected by Rust to Scheme & UI.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitSnapshot {
    pub revision: u64,
    pub repo_name: String,
    pub repo_root: String,
    pub backend: MagitBackendKind,
    pub branch: String,
    pub upstream: Option<String>,
    pub latest_tag: Option<String>,
    pub head_sha: Option<MagitCommitSha>,
    pub change_id: Option<MagitChangeId>,
    pub head_description: String,
    pub ahead: u32,
    pub behind: u32,
    pub conflicts: Vec<MagitFileEntry>,
    pub unstaged: Vec<MagitFileEntry>,
    pub staged: Vec<MagitFileEntry>,
    pub untracked: Vec<MagitFileEntry>,
    pub stashes: Vec<MagitStashEntry>,
    pub unpushed: Vec<MagitLogEntry>,
    pub unpulled: Vec<MagitLogEntry>,
    pub recent_log: Vec<MagitLogEntry>,
    pub op_log: Vec<MagitOpLogEntry>,
    pub aitia: MagitAitiaProvenance,
}

/// Typed Magit mutation request admitted by Rust (synced with `magit.el`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MagitAction {
    Refresh,
    StageFile { path: String },
    UnstageFile { path: String },
    StageHunk { path: String, patch: String },
    UnstageHunk { path: String, patch: String },
    DiscardHunk { path: String, patch: String },
    StageAll,
    UnstageAll,
    DiscardFile { path: String },
    Commit { message: String, amend: bool },
    ExtendHead,
    JjDescribe { message: String },
    JjNew { message: Option<String> },
    JjSquash,
    JjUndo,
    CheckoutBranch { name: String, create: bool },
    DeleteBranch { name: String },
    StashPush { message: Option<String> },
    StashPop,
    StashApply,
    StashDrop,
    Fetch { all: bool },
    Pull { rebase: bool },
    Push,
    ResetHead { mode: String, target: String },
    CherryPick { rev: String },
    RevertCommit { rev: String },
    RecordJointAudit {
        hunk_id: String,
        actor: String,
        verdict: String,
        rationale: String,
    },
}

/// Typed execution receipt minted after a Magit mutation (bound to Aitia evidence).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MagitActionReceipt {
    pub action: String,
    pub backend: MagitBackendKind,
    pub ok: bool,
    pub summary: String,
    pub op_id_before: Option<MagitOpId>,
    pub op_id_after: Option<MagitOpId>,
    pub commit_sha: Option<MagitCommitSha>,
    pub aitia_evidence_id: String,
    pub requires_gate: bool,
}
