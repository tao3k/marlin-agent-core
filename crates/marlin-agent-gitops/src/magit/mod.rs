// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! AI-Native Magit engine for Marlin Agent Core.
//!
//! Unifies bounded, SM32-hardened Git and colocated Jujutsu (`jj`) operations
//! (including hunk-level apply/stage/unstage/discard, stashes, fetch/pull/push,
//! reset, cherry-pick, revert) with REAL Lambda Aitia gRPC defense audit
//! (`aitia.marlin.v1.AitiaSidecarGateway`) and Human-AI joint evidence chains.

pub mod aitia_client;
pub mod diff_parser;
pub mod engine;
pub mod types;

pub use aitia_client::{
    AitiaFormalModelRecommendation, AitiaFormalOverlayProjection, AitiaG2Finding, AitiaLiveReport,
    AitiaMarlinGrpcClient, AitiaProgressiveReviewSnapshot, AitiaStepReviewRequest,
};
pub use diff_parser::{
    build_single_hunk_patch, classify_structural_concern, fuse_hunk_evidence, parse_unified_diff,
};
pub use engine::{MagitEngine, MagitError, format_hunk_preview, summarize_concerns};
pub use types::{
    MagitAction, MagitActionReceipt, MagitAitiaProvenance, MagitAuditConcern, MagitAuditRisk,
    MagitBackendKind, MagitChangeId, MagitCommitSha, MagitDiffHunk, MagitDiffLine,
    MagitDiffLineKind, MagitEvidenceItem, MagitEvidenceSource, MagitFileEntry, MagitFileSection,
    MagitFormalStatus, MagitJointAuditRecord, MagitLogEntry, MagitOpId, MagitOpLogEntry,
    MagitSnapshot, MagitStashEntry,
};
