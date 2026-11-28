// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Live Rust-owned AI-Native Magit bridge (`jj` + Git + REAL Lambda Aitia gRPC +
//! Human-AI Joint Evidence Chain) for `marlin-agent-app-server`.
//!
//! Projects typed `MagitSnapshot`, real `AitiaLiveReport` findings, stashes,
//! unpushed/unpulled commits, and `MagitActionReceipt` records into
//! `GraphRecordView` + `WorkspaceDiffSnapshot` + `GraphTrajectoryEvent` with
//! zero breaking `.proto` changes.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use super::bridge_framework::{BridgeCommandOutcome, PackageBridge, SwrCache};

use marlin_agent_gitops::magit::{
    AitiaFormalOverlayProjection, AitiaMarlinGrpcClient, AitiaProgressiveReviewSnapshot,
    AitiaStepReviewRequest, MagitAction, MagitActionReceipt, MagitDiffHunk, MagitDiffLineKind,
    MagitEngine, MagitFileEntry, MagitJointAuditRecord, MagitLogEntry, MagitSnapshot,
    build_single_hunk_patch,
};

use crate::wire::marlin::projection::v1::{
    FileDiffEntry, GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface,
    WorkspaceDiffSnapshot,
};

const MAX_HUNK_PREVIEW_LINES: usize = 48;
const MAGIT_SNAPSHOT_TTL: Duration = Duration::from_secs(3);

/// Thread-safe bridge hosting a workspace's `MagitEngine`, persistent RFC 0024
/// Lambda Aitia gRPC client (`aitia.marlin.v1.AitiaSidecarGateway`), Human-AI
/// joint audit ledger, and cached snapshot/receipt projections.
#[derive(Clone)]
pub struct WorkspaceMagitBridge {
    workspace_root: PathBuf,
    revision: Arc<AtomicU64>,
    aitia_client: AitiaMarlinGrpcClient,
    joint_audits: Arc<RwLock<Vec<MagitJointAuditRecord>>>,
    cached_snapshot: Arc<RwLock<Option<MagitSnapshot>>>,
    snapshot_cache: SwrCache<PathBuf, MagitSnapshot>,
    last_receipt: Arc<RwLock<Option<MagitActionReceipt>>>,
    latest_review: Arc<RwLock<Option<AitiaProgressiveReviewSnapshot>>>,
}

impl WorkspaceMagitBridge {
    /// Creates a new bridge bound to `workspace_root` with auto-managed Lambda Aitia gRPC.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            revision: Arc::new(AtomicU64::new(0)),
            aitia_client: AitiaMarlinGrpcClient::from_env(),
            joint_audits: Arc::new(RwLock::new(Vec::new())),
            cached_snapshot: Arc::new(RwLock::new(None)),
            snapshot_cache: SwrCache::new(MAGIT_SNAPSHOT_TTL),
            last_receipt: Arc::new(RwLock::new(None)),
            latest_review: Arc::new(RwLock::new(None)),
        }
    }

    /// Pre-warms the default workspace `MagitSnapshot` on a background Tokio task
    /// via the shared `SwrCache` so the very first `*magit*` open is instant.
    pub fn spawn_prewarm(&self) {
        let bridge = self.clone();
        self.snapshot_cache
            .spawn_background(self.workspace_root.clone(), move |dir| async move {
                bridge.refresh_snapshot_for(Some(&dir)).await
            });
    }

    /// Returns a fast `MagitSnapshot` via the shared framework `SwrCache`:
    /// - `!force_refresh` + cached entry -> immediate return (< 0.1ms) +
    ///   coalesced background refresh when older than `MAGIT_SNAPSHOT_TTL`.
    /// - `force_refresh` or cache miss -> awaits an authoritative refresh.
    pub async fn snapshot_fast_for(
        &self,
        repo_override: Option<&Path>,
        force_refresh: bool,
    ) -> Result<MagitSnapshot, String> {
        let target_dir = repo_override.unwrap_or(&self.workspace_root).to_path_buf();
        let bg_bridge = self.clone();
        self.snapshot_cache
            .get_or_load(
                target_dir,
                force_refresh,
                || self.refresh_snapshot_for(repo_override),
                move |dir| async move { bg_bridge.refresh_snapshot_for(Some(&dir)).await },
            )
            .await
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns the most recently cached snapshot without spawning a subprocess.
    pub fn cached_snapshot(&self) -> Option<MagitSnapshot> {
        self.cached_snapshot
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Returns the most recent mutation receipt, if any.
    pub fn last_receipt(&self) -> Option<MagitActionReceipt> {
        self.last_receipt
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Stores a verified zero-authority `AitiaProgressiveReviewSnapshot` for UI Magit projection.
    pub fn record_progressive_review(&self, review: AitiaProgressiveReviewSnapshot) {
        if let Ok(mut guard) = self.latest_review.write() {
            *guard = Some(review);
        }
    }

    /// Returns the most recent cached progressive FunFlow review snapshot, if any.
    pub fn latest_progressive_review(&self) -> Option<AitiaProgressiveReviewSnapshot> {
        self.latest_review
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Runs a live read-only `AitiaSidecarGateway::StepReview` round over the workspace
    /// (or `repo_override`) and caches the resulting zero-authority disposition snapshot.
    pub async fn run_progressive_review_for(
        &self,
        repo_override: Option<&Path>,
        vault_dir: Option<&Path>,
    ) -> Result<AitiaProgressiveReviewSnapshot, String> {
        let target_dir = repo_override.unwrap_or(&self.workspace_root);
        let engine = MagitEngine::discover(target_dir)
            .await
            .map_err(|e| e.to_string())?;
        let req = AitiaStepReviewRequest {
            repo_dir: engine.repo_root().to_path_buf(),
            base_ref: "HEAD".to_owned(),
            vault_dir: vault_dir.map(Path::to_path_buf),
            ..Default::default()
        };
        let review = self.aitia_client.step_review(&req).await?;
        self.record_progressive_review(review.clone());
        Ok(review)
    }

    /// Runs a typed `AitiaSidecarGateway::StepReview` request and caches its snapshot.
    pub async fn step_progressive_review(
        &self,
        req: &AitiaStepReviewRequest,
    ) -> Result<AitiaProgressiveReviewSnapshot, String> {
        let review = self.aitia_client.step_review(req).await?;
        self.record_progressive_review(review.clone());
        Ok(review)
    }

    fn current_joint_audits(&self) -> Vec<MagitJointAuditRecord> {
        self.joint_audits
            .read()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Computes a fresh authoritative `MagitSnapshot` from `jj` / Git fused with
    /// real Lambda Aitia gRPC `AuditDiffDefense` findings and Human-AI audit records.
    pub async fn refresh_snapshot(&self) -> Result<MagitSnapshot, String> {
        self.refresh_snapshot_for(None).await
    }

    /// Computes a fresh authoritative `MagitSnapshot` for `repo_override` (if provided)
    /// or the bridge's default `workspace_root`, discovering the live Git/jj worktree.
    pub async fn refresh_snapshot_for(
        &self,
        repo_override: Option<&Path>,
    ) -> Result<MagitSnapshot, String> {
        let target_dir = repo_override.unwrap_or(&self.workspace_root);
        let engine = MagitEngine::discover(target_dir)
            .await
            .map_err(|e| e.to_string())?;
        let rev = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let aitia_report = self
            .aitia_client
            .audit_diff_defense(engine.repo_root(), "HEAD")
            .await;
        let joints = self.current_joint_audits();
        let snap = engine
            .snapshot_with_evidence(rev, &aitia_report, &joints)
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(mut guard) = self.cached_snapshot.write()
            && repo_override.is_none()
        {
            *guard = Some(snap.clone());
        }
        self.snapshot_cache.insert_keys(
            [target_dir.to_path_buf(), engine.repo_root().to_path_buf()],
            snap.clone(),
        );
        Ok(snap)
    }

    /// Parses and executes a `magit.*` declarative UI command through `MagitEngine`
    /// (including hunk-level stage/unstage/discard and joint audit recording).
    /// Returns `Ok(None)` when `raw_command` is not a `magit.*` command.
    pub async fn execute_ui_command(
        &self,
        raw_command: &str,
    ) -> Result<Option<(MagitActionReceipt, MagitSnapshot, GraphTrajectoryEvent)>, String> {
        let cmd = raw_command.trim();
        if !cmd.starts_with("magit.") {
            return Ok(None);
        }

        // Pure read-only progressive FunFlow review step (RFC 0008 / RFC 0026).
        // Never mutates Git/jj and always preserves actionAuthorized=false.
        if cmd == "magit.aitia.review" || cmd.starts_with("magit.aitia.review ") {
            let vault_dir = cmd
                .strip_prefix("magit.aitia.review")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from);
            let review = self
                .run_progressive_review_for(None, vault_dir.as_deref())
                .await?;
            let snap = self.refresh_snapshot().await?;
            let receipt = MagitActionReceipt {
                action: "magit.aitia.review".to_owned(),
                backend: snap.backend,
                ok: true,
                summary: format!(
                    "ASCENT choice={} worlds={} admissible={} cas_hit={} open_obligations={}",
                    review.choice_status,
                    review.world_count,
                    review.admissible,
                    review.shared_prefix_cas_hit,
                    review.open_obligations.len(),
                ),
                op_id_before: snap.aitia.op_id.clone(),
                op_id_after: snap.aitia.op_id.clone(),
                commit_sha: snap.head_sha.clone(),
                aitia_evidence_id: format!("aitia-review:{}", review.context_digest),
                requires_gate: review.human_disposition_required,
            };
            if let Ok(mut guard) = self.last_receipt.write() {
                *guard = Some(receipt.clone());
            }
            let traj = receipt_to_trajectory_event(&receipt, &snap);
            return Ok(Some((receipt, snap, traj)));
        }

        // Resolve hunk-level commands against the cached (or freshly refreshed) snapshot.
        let action = if let Some(hunk_id) = cmd.strip_prefix("magit.stage-hunk ") {
            let hunk = self.resolve_hunk(hunk_id.trim()).await?;
            MagitAction::StageHunk {
                path: hunk.file_path.clone(),
                patch: build_single_hunk_patch(&hunk),
            }
        } else if let Some(hunk_id) = cmd.strip_prefix("magit.unstage-hunk ") {
            let hunk = self.resolve_hunk(hunk_id.trim()).await?;
            MagitAction::UnstageHunk {
                path: hunk.file_path.clone(),
                patch: build_single_hunk_patch(&hunk),
            }
        } else if let Some(hunk_id) = cmd.strip_prefix("magit.discard-hunk ") {
            let hunk = self.resolve_hunk(hunk_id.trim()).await?;
            MagitAction::DiscardHunk {
                path: hunk.file_path.clone(),
                patch: build_single_hunk_patch(&hunk),
            }
        } else {
            let Some(parsed) = parse_magit_ui_command(cmd)? else {
                return Ok(None);
            };
            parsed
        };

        if let MagitAction::RecordJointAudit {
            ref hunk_id,
            ref actor,
            ref verdict,
            ref rationale,
        } = action
        {
            let digest = self
                .cached_snapshot()
                .and_then(|s| s.aitia.aitia_report_digest);
            if let Ok(mut guard) = self.joint_audits.write() {
                guard.retain(|r| !(r.hunk_id == *hunk_id && r.actor == *actor));
                if verdict != "unreviewed" {
                    guard.push(MagitJointAuditRecord {
                        hunk_id: hunk_id.clone(),
                        actor: actor.clone(),
                        verdict: verdict.clone(),
                        rationale: rationale.clone(),
                        context_digest: digest,
                    });
                }
            }
        }

        let engine = MagitEngine::discover(&self.workspace_root)
            .await
            .map_err(|e| e.to_string())?;
        let receipt = engine.execute(action).await.map_err(|e| e.to_string())?;
        let snap = self.refresh_snapshot().await?;
        if let Ok(mut guard) = self.last_receipt.write() {
            *guard = Some(receipt.clone());
        }
        let traj = receipt_to_trajectory_event(&receipt, &snap);
        Ok(Some((receipt, snap, traj)))
    }

    async fn resolve_hunk(&self, hunk_id: &str) -> Result<MagitDiffHunk, String> {
        if hunk_id.is_empty() {
            return Err("hunk command requires a non-empty hunk_id".to_owned());
        }
        let snap = match self.cached_snapshot() {
            Some(s) => s,
            None => self.refresh_snapshot().await?,
        };
        for list in [&snap.conflicts, &snap.unstaged, &snap.staged] {
            for f in list {
                for h in &f.hunks {
                    if h.hunk_id == hunk_id {
                        return Ok(h.clone());
                    }
                }
            }
        }
        Err(format!("hunk {hunk_id:?} not found in current snapshot"))
    }

    /// Projects a `MagitSnapshot` (plus optional latest mutation receipt) into
    /// structured `GraphRecordView` rows for `ProjectionQueryService::query_project_graph`.
    pub fn project_snapshot(
        snap: &MagitSnapshot,
        receipt: Option<&MagitActionReceipt>,
    ) -> Vec<GraphRecordView> {
        Self::project_snapshot_with_review(snap, receipt, None)
    }

    /// Projects a `MagitSnapshot`, optional mutation receipt, and optional RFC 0008 /
    /// RFC 0026 `AitiaProgressiveReviewSnapshot` into structured `GraphRecordView` rows.
    pub fn project_snapshot_with_review(
        snap: &MagitSnapshot,
        receipt: Option<&MagitActionReceipt>,
        review: Option<&AitiaProgressiveReviewSnapshot>,
    ) -> Vec<GraphRecordView> {
        let mut records = Vec::new();

        // 1. Repository head, upstream, tag & REAL Lambda Aitia gRPC telemetry.
        let mut head_facets = HashMap::new();
        head_facets.insert("revision".to_owned(), snap.revision.to_string());
        head_facets.insert("backend".to_owned(), snap.backend.as_str().to_owned());
        head_facets.insert("branch".to_owned(), snap.branch.clone());
        if let Some(ref up) = snap.upstream {
            head_facets.insert("upstream".to_owned(), up.clone());
        }
        if let Some(ref tag) = snap.latest_tag {
            head_facets.insert("latest_tag".to_owned(), tag.clone());
        }
        if let Some(ref sha) = snap.head_sha {
            head_facets.insert("head_sha".to_owned(), sha.as_str().to_owned());
        }
        if let Some(ref cid) = snap.change_id {
            head_facets.insert("change_id".to_owned(), cid.as_str().to_owned());
        }
        head_facets.insert("ahead".to_owned(), snap.ahead.to_string());
        head_facets.insert("behind".to_owned(), snap.behind.to_string());
        head_facets.insert(
            "aitia_evidence_id".to_owned(),
            snap.aitia.evidence_id.clone(),
        );
        head_facets.insert(
            "aitia_online".to_owned(),
            if snap.aitia.aitia_online {
                "true"
            } else {
                "false"
            }
            .to_owned(),
        );
        head_facets.insert(
            "aitia_decision".to_owned(),
            snap.aitia.aitia_decision.clone(),
        );
        head_facets.insert(
            "aitia_engine_owner".to_owned(),
            snap.aitia.aitia_engine_owner.clone(),
        );
        if let Some(ref digest) = snap.aitia.aitia_report_digest {
            head_facets.insert("aitia_report_digest".to_owned(), digest.clone());
        }
        if let Some(acyc) = snap.aitia.causal_acyclic {
            head_facets.insert(
                "causal_acyclic".to_owned(),
                if acyc { "true" } else { "false" }.to_owned(),
            );
        }
        head_facets.insert(
            "aitia_total_findings".to_owned(),
            snap.aitia.aitia_total_findings.to_string(),
        );
        head_facets.insert("aitia_note".to_owned(), snap.aitia.aitia_note.clone());
        if let Some(ref op) = snap.aitia.op_id {
            head_facets.insert("op_id".to_owned(), op.as_str().to_owned());
        }
        head_facets.insert(
            "high_risk_hunks".to_owned(),
            snap.aitia.high_risk_hunks.to_string(),
        );
        head_facets.insert(
            "medium_risk_hunks".to_owned(),
            snap.aitia.medium_risk_hunks.to_string(),
        );
        head_facets.insert("total_hunks".to_owned(), snap.aitia.total_hunks.to_string());
        head_facets.insert(
            "conflict_count".to_owned(),
            snap.aitia.conflict_count.to_string(),
        );

        records.push(GraphRecordView {
            record_id: format!("magit-head:r{}", snap.revision),
            surface: GraphVisibleSurface::Workspace as i32,
            kind: "magit.head".to_owned(),
            title: snap.repo_name.clone(),
            summary: snap.head_description.clone(),
            source_path: Some(snap.repo_root.clone()),
            start_line: None,
            end_line: None,
            facets: head_facets,
        });

        // 2. Latest mutation receipt.
        if let Some(r) = receipt {
            let mut rf = HashMap::new();
            rf.insert(
                "ok".to_owned(),
                if r.ok { "true" } else { "false" }.to_owned(),
            );
            rf.insert("aitia_evidence_id".to_owned(), r.aitia_evidence_id.clone());
            if let Some(ref before) = r.op_id_before {
                rf.insert("op_id_before".to_owned(), before.as_str().to_owned());
            }
            if let Some(ref after) = r.op_id_after {
                rf.insert("op_id_after".to_owned(), after.as_str().to_owned());
            }
            records.push(GraphRecordView {
                record_id: format!("magit-receipt:{}", r.aitia_evidence_id),
                surface: GraphVisibleSurface::Evidence as i32,
                kind: "magit.receipt".to_owned(),
                title: r.action.clone(),
                summary: r.summary.clone(),
                source_path: Some(snap.repo_root.clone()),
                start_line: None,
                end_line: None,
                facets: rf,
            });
        }

        // 3. Lambda Aitia causal evidence binding record.
        let mut af = HashMap::new();
        af.insert("backend".to_owned(), snap.aitia.backend.as_str().to_owned());
        af.insert(
            "aitia_online".to_owned(),
            if snap.aitia.aitia_online {
                "true"
            } else {
                "false"
            }
            .to_owned(),
        );
        af.insert("decision".to_owned(), snap.aitia.aitia_decision.clone());
        af.insert(
            "engine_owner".to_owned(),
            snap.aitia.aitia_engine_owner.clone(),
        );
        af.insert(
            "total_findings".to_owned(),
            snap.aitia.aitia_total_findings.to_string(),
        );
        if let Some(ref digest) = snap.aitia.aitia_report_digest {
            af.insert("report_digest".to_owned(), digest.clone());
        }
        records.push(GraphRecordView {
            record_id: snap.aitia.evidence_id.clone(),
            surface: GraphVisibleSurface::Evidence as i32,
            kind: "aitia.evidence".to_owned(),
            title: format!(
                "Lambda Aitia · {} ({})",
                snap.aitia.aitia_decision, snap.aitia.aitia_engine_owner
            ),
            summary: snap.aitia.aitia_note.clone(),
            source_path: Some(snap.repo_root.clone()),
            start_line: None,
            end_line: None,
            facets: af,
        });

        // 4. Changed files & parsed diff hunks with cited evidence chains.
        for file in snap
            .conflicts
            .iter()
            .chain(snap.unstaged.iter())
            .chain(snap.staged.iter())
            .chain(snap.untracked.iter())
        {
            push_file_records(&mut records, file);
        }

        // 5. Git Stashes (`magit-insert-stashes`).
        for (idx, st) in snap.stashes.iter().enumerate() {
            let mut sf = HashMap::new();
            sf.insert("reference".to_owned(), st.reference.clone());
            sf.insert("relative_time".to_owned(), st.relative_time.clone());
            records.push(GraphRecordView {
                record_id: format!("magit-stash:{idx}:{}", st.reference),
                surface: GraphVisibleSurface::Workspace as i32,
                kind: "magit.stash".to_owned(),
                title: st.summary.clone(),
                summary: st.relative_time.clone(),
                source_path: None,
                start_line: None,
                end_line: None,
                facets: sf,
            });
        }

        // 6. Unpushed & Unpulled commits (`magit-status.el`).
        for (idx, entry) in snap.unpushed.iter().enumerate() {
            push_log_record(&mut records, "magit.unpushed", idx, entry);
        }
        for (idx, entry) in snap.unpulled.iter().enumerate() {
            push_log_record(&mut records, "magit.unpulled", idx, entry);
        }

        // 7. Recent commits / jj changes.
        for (idx, entry) in snap.recent_log.iter().enumerate() {
            push_log_record(&mut records, "magit.log", idx, entry);
        }

        // 8. Jujutsu operation log entries.
        for (idx, op) in snap.op_log.iter().enumerate() {
            let mut of = HashMap::new();
            of.insert("op_id".to_owned(), op.op_id.as_str().to_owned());
            of.insert(
                "is_current".to_owned(),
                if op.is_current { "true" } else { "false" }.to_owned(),
            );
            records.push(GraphRecordView {
                record_id: format!("magit-op:{idx}:{}", op.op_id.as_str()),
                surface: GraphVisibleSurface::Workspace as i32,
                kind: "magit.oplog".to_owned(),
                title: op.description.clone(),
                summary: op.timestamp.clone(),
                source_path: None,
                start_line: None,
                end_line: None,
                facets: of,
            });
        }

        // 9. Optional RFC 0008 / RFC 0026 progressive FunFlow review & formal vault overlays.
        if let Some(rev) = review {
            records.extend(Self::project_progressive_review(rev, &snap.repo_root));
        }

        records
    }

    /// Projects a verified zero-authority `AitiaProgressiveReviewSnapshot` into read-only
    /// `GraphRecordView` rows for the Magit human disposition panel (zero `.proto` changes).
    pub fn project_progressive_review(
        review: &AitiaProgressiveReviewSnapshot,
        repo_root: &str,
    ) -> Vec<GraphRecordView> {
        let mut records = Vec::new();
        let mut rf = HashMap::new();
        rf.insert("choice_status".to_owned(), review.choice_status.clone());
        rf.insert("world_count".to_owned(), review.world_count.to_string());
        rf.insert(
            "admissible".to_owned(),
            if review.admissible { "true" } else { "false" }.to_owned(),
        );
        rf.insert("action_authorized".to_owned(), "false".to_owned());
        rf.insert(
            "human_disposition_required".to_owned(),
            if review.human_disposition_required {
                "true"
            } else {
                "false"
            }
            .to_owned(),
        );
        rf.insert(
            "shared_prefix_cas_hit".to_owned(),
            if review.shared_prefix_cas_hit {
                "true"
            } else {
                "false"
            }
            .to_owned(),
        );
        rf.insert("plan_format".to_owned(), review.plan_format.clone());
        rf.insert(
            "plan_valid".to_owned(),
            if review.plan_valid { "true" } else { "false" }.to_owned(),
        );
        rf.insert(
            "plan_acyclic".to_owned(),
            if review.plan_acyclic { "true" } else { "false" }.to_owned(),
        );
        rf.insert("context_digest".to_owned(), review.context_digest.clone());
        rf.insert(
            "open_obligations".to_owned(),
            review.open_obligations.join(","),
        );
        rf.insert(
            "active_rule_packs".to_owned(),
            review.active_rule_packs.join(","),
        );
        rf.insert(
            "findings_count".to_owned(),
            review.findings.len().to_string(),
        );

        records.push(GraphRecordView {
            record_id: format!("aitia-review:{}", review.context_digest),
            surface: GraphVisibleSurface::Evidence as i32,
            kind: "aitia.review.step".to_owned(),
            title: format!(
                "Aitia FunFlow Review · {} (worlds={})",
                review.choice_status, review.world_count
            ),
            summary: format!(
                "admissible={}, actionAuthorized=false, openObligations=[{}]",
                review.admissible,
                review.open_obligations.join(", ")
            ),
            source_path: Some(repo_root.to_owned()),
            start_line: None,
            end_line: None,
            facets: rf,
        });

        for overlay in [&review.quint_overlay, &review.lean_overlay]
            .into_iter()
            .flatten()
        {
            records.push(project_overlay_record(overlay, repo_root));
        }

        for rec in &review.recommended_formal_models {
            let mut mf = HashMap::new();
            mf.insert("model_id".to_owned(), rec.model_id.clone());
            mf.insert("tool".to_owned(), rec.tool.clone());
            mf.insert("library".to_owned(), rec.library.clone());
            records.push(GraphRecordView {
                record_id: format!("aitia-formal-rec:{}:{}", rec.tool, rec.model_id),
                surface: GraphVisibleSurface::Evidence as i32,
                kind: "aitia.formal.recommendation".to_owned(),
                title: format!("{} ({})", rec.model_id, rec.tool),
                summary: rec.rationale.clone(),
                source_path: Some(repo_root.to_owned()),
                start_line: None,
                end_line: None,
                facets: mf,
            });
        }

        records
    }

    /// Converts a `MagitSnapshot` into the streaming `WorkspaceDiffSnapshot` proto.
    pub fn to_workspace_diff_snapshot(snap: &MagitSnapshot) -> WorkspaceDiffSnapshot {
        let mut files = Vec::new();
        for f in snap
            .conflicts
            .iter()
            .chain(snap.staged.iter())
            .chain(snap.unstaged.iter())
            .chain(snap.untracked.iter())
        {
            files.push(FileDiffEntry {
                path: f.path.clone(),
                status: f.status_code.clone(),
                added_lines: f.added,
                removed_lines: f.removed,
                diff_artifact: None,
            });
        }
        WorkspaceDiffSnapshot {
            revision: snap.revision.max(1),
            branch: snap.branch.clone(),
            files,
        }
    }
}

#[tonic::async_trait]
impl PackageBridge for WorkspaceMagitBridge {
    fn namespace(&self) -> &'static str {
        "magit"
    }

    fn owner_id(&self) -> &'static str {
        "marlin.magit"
    }

    fn prewarm(&self) {
        self.spawn_prewarm();
    }

    async fn query(
        &self,
        verb: &str,
        arg: Option<&str>,
        _raw_query: &str,
    ) -> Result<Vec<GraphRecordView>, String> {
        let repo_override = arg.map(PathBuf::from);
        if verb == "aitia-review" {
            let review = if let Some(ref repo) = repo_override {
                self.run_progressive_review_for(Some(repo.as_path()), None)
                    .await
                    .map_err(|err| format!("aitia review error: {err}"))?
            } else if let Some(cached) = self.latest_progressive_review() {
                cached
            } else {
                self.run_progressive_review_for(None, None)
                    .await
                    .map_err(|err| format!("aitia review error: {err}"))?
            };
            let root_str = repo_override
                .as_deref()
                .unwrap_or_else(|| self.workspace_root())
                .to_string_lossy()
                .into_owned();
            return Ok(Self::project_progressive_review(&review, &root_str));
        }

        let force_refresh = verb == "refresh";
        let snap = self
            .snapshot_fast_for(repo_override.as_deref(), force_refresh)
            .await
            .map_err(|err| format!("magit snapshot error: {err}"))?;
        let receipt = self.last_receipt();
        let review = self.latest_progressive_review();
        Ok(Self::project_snapshot_with_review(
            &snap,
            receipt.as_ref(),
            review.as_ref(),
        ))
    }

    async fn execute_command(
        &self,
        raw_command: &str,
    ) -> Result<Option<BridgeCommandOutcome>, String> {
        let Some((receipt, _snap, traj)) = self.execute_ui_command(raw_command).await? else {
            return Ok(None);
        };
        Ok(Some(BridgeCommandOutcome {
            ok: receipt.ok,
            summary: receipt.summary,
            owner_id: self.owner_id().to_owned(),
            trajectory: Some(traj),
        }))
    }

    async fn workspace_diff(&self) -> Option<WorkspaceDiffSnapshot> {
        let snap = self.snapshot_fast_for(None, false).await.ok()?;
        Some(Self::to_workspace_diff_snapshot(&snap))
    }
}

fn push_log_record(
    records: &mut Vec<GraphRecordView>,
    kind: &str,
    idx: usize,
    entry: &MagitLogEntry,
) {
    let mut lf = HashMap::new();
    lf.insert(
        "commit_sha".to_owned(),
        entry.commit_sha.as_str().to_owned(),
    );
    if let Some(ref cid) = entry.change_id {
        lf.insert("change_id".to_owned(), cid.as_str().to_owned());
    }
    if !entry.bookmarks.is_empty() {
        lf.insert("bookmarks".to_owned(), entry.bookmarks.join(","));
    }
    lf.insert("author".to_owned(), entry.author.clone());
    lf.insert("relative_time".to_owned(), entry.relative_time.clone());
    lf.insert(
        "has_conflict".to_owned(),
        if entry.has_conflict { "true" } else { "false" }.to_owned(),
    );
    lf.insert(
        "is_working_copy".to_owned(),
        if entry.is_working_copy {
            "true"
        } else {
            "false"
        }
        .to_owned(),
    );
    records.push(GraphRecordView {
        record_id: format!("{kind}:{idx}:{}", entry.commit_sha.as_str()),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: kind.to_owned(),
        title: entry.summary.clone(),
        summary: format!("{} · {}", entry.author, entry.relative_time),
        source_path: None,
        start_line: None,
        end_line: None,
        facets: lf,
    });
}

fn push_file_records(records: &mut Vec<GraphRecordView>, file: &MagitFileEntry) {
    let mut ff = HashMap::new();
    ff.insert("section".to_owned(), file.section.as_str().to_owned());
    ff.insert("status_code".to_owned(), file.status_code.clone());
    ff.insert("added".to_owned(), file.added.to_string());
    ff.insert("removed".to_owned(), file.removed.to_string());
    ff.insert("risk".to_owned(), file.risk.as_str().to_owned());
    ff.insert("concern".to_owned(), file.concern.label().to_owned());
    if let Some(ref orig) = file.original_path {
        ff.insert("original_path".to_owned(), orig.clone());
    }
    records.push(GraphRecordView {
        record_id: format!("magit-file:{}:{}", file.section.as_str(), file.path),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "magit.file".to_owned(),
        title: file.path.clone(),
        summary: file.risk_reason.clone(),
        source_path: Some(file.path.clone()),
        start_line: None,
        end_line: None,
        facets: ff,
    });

    for hunk in &file.hunks {
        push_hunk_record(records, file.section.as_str(), hunk);
    }
}

fn push_hunk_record(records: &mut Vec<GraphRecordView>, section: &str, hunk: &MagitDiffHunk) {
    let mut hf = HashMap::new();
    hf.insert("section".to_owned(), section.to_owned());
    hf.insert("added".to_owned(), hunk.added.to_string());
    hf.insert("removed".to_owned(), hunk.removed.to_string());
    hf.insert("risk".to_owned(), hunk.risk.as_str().to_owned());
    hf.insert("concern".to_owned(), hunk.concern.label().to_owned());
    hf.insert(
        "formal_status".to_owned(),
        hunk.formal_status.as_str().to_owned(),
    );
    hf.insert("risk_reason".to_owned(), hunk.risk_reason.clone());
    hf.insert("old_start".to_owned(), hunk.old_start.to_string());
    hf.insert("old_count".to_owned(), hunk.old_count.to_string());
    hf.insert("new_start".to_owned(), hunk.new_start.to_string());
    hf.insert("new_count".to_owned(), hunk.new_count.to_string());

    // Project structured cited evidence chain onto the hunk record.
    let ev_lines: Vec<String> = hunk
        .evidence
        .iter()
        .map(|ev| {
            format!(
                "{}|{}|{}|{}",
                ev.source.as_str(),
                ev.verdict,
                ev.rule_or_actor,
                ev.summary.replace('\n', " ")
            )
        })
        .collect();
    if !ev_lines.is_empty() {
        hf.insert("evidence_chain".to_owned(), ev_lines.join("\n"));
    }

    let mut preview = String::new();
    for line in hunk.lines.iter().take(MAX_HUNK_PREVIEW_LINES) {
        let prefix = match line.kind {
            MagitDiffLineKind::Added => '+',
            MagitDiffLineKind::Removed => '-',
            MagitDiffLineKind::Context => ' ',
        };
        preview.push(prefix);
        preview.push_str(&line.text);
        preview.push('\n');
    }
    if hunk.lines.len() > MAX_HUNK_PREVIEW_LINES {
        preview.push_str(&format!(
            " ... ({} more lines)\n",
            hunk.lines.len() - MAX_HUNK_PREVIEW_LINES
        ));
    }

    records.push(GraphRecordView {
        record_id: hunk.hunk_id.clone(),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: "magit.hunk".to_owned(),
        title: hunk.header.clone(),
        summary: preview.trim_end_matches('\n').to_owned(),
        source_path: Some(hunk.file_path.clone()),
        start_line: Some(hunk.new_start),
        end_line: Some(hunk.new_start.saturating_add(hunk.new_count)),
        facets: hf,
    });
}

/// Parses a declarative `magit.*` UI command string into a typed `MagitAction`.
pub fn parse_magit_ui_command(raw: &str) -> Result<Option<MagitAction>, String> {
    let cmd = raw.trim();
    if !cmd.starts_with("magit.") {
        return Ok(None);
    }
    match cmd {
        "magit.refresh" => return Ok(Some(MagitAction::Refresh)),
        "magit.stage-all" => return Ok(Some(MagitAction::StageAll)),
        "magit.unstage-all" => return Ok(Some(MagitAction::UnstageAll)),
        "magit.extend" => return Ok(Some(MagitAction::ExtendHead)),
        "magit.jj.undo" => return Ok(Some(MagitAction::JjUndo)),
        "magit.jj.new" => return Ok(Some(MagitAction::JjNew { message: None })),
        "magit.jj.squash" => return Ok(Some(MagitAction::JjSquash)),
        "magit.stash.push" => return Ok(Some(MagitAction::StashPush { message: None })),
        "magit.stash.pop" => return Ok(Some(MagitAction::StashPop)),
        "magit.stash.apply" => return Ok(Some(MagitAction::StashApply)),
        "magit.stash.drop" => return Ok(Some(MagitAction::StashDrop)),
        "magit.fetch" => return Ok(Some(MagitAction::Fetch { all: false })),
        "magit.fetch.all" => return Ok(Some(MagitAction::Fetch { all: true })),
        "magit.pull" => return Ok(Some(MagitAction::Pull { rebase: false })),
        "magit.pull.rebase" => return Ok(Some(MagitAction::Pull { rebase: true })),
        "magit.push" => return Ok(Some(MagitAction::Push)),
        "magit.reset.soft" => {
            return Ok(Some(MagitAction::ResetHead {
                mode: "soft".to_owned(),
                target: "HEAD~1".to_owned(),
            }));
        }
        "magit.reset.mixed" => {
            return Ok(Some(MagitAction::ResetHead {
                mode: "mixed".to_owned(),
                target: "HEAD~1".to_owned(),
            }));
        }
        _ => {}
    }

    if let Some(arg) = cmd.strip_prefix("magit.stage ") {
        let path = arg.trim();
        if path.is_empty() {
            return Err("magit.stage requires a file path".to_owned());
        }
        return Ok(Some(MagitAction::StageFile {
            path: path.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.unstage ") {
        let path = arg.trim();
        if path.is_empty() {
            return Err("magit.unstage requires a file path".to_owned());
        }
        return Ok(Some(MagitAction::UnstageFile {
            path: path.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.discard ") {
        let path = arg.trim();
        if path.is_empty() {
            return Err("magit.discard requires a file path".to_owned());
        }
        return Ok(Some(MagitAction::DiscardFile {
            path: path.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.commit ") {
        let message = arg.trim();
        if message.is_empty() {
            return Err("magit.commit requires a non-empty message".to_owned());
        }
        return Ok(Some(MagitAction::Commit {
            message: message.to_owned(),
            amend: false,
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.amend ") {
        let message = arg.trim();
        if message.is_empty() {
            return Err("magit.amend requires a non-empty message".to_owned());
        }
        return Ok(Some(MagitAction::Commit {
            message: message.to_owned(),
            amend: true,
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.describe ") {
        let message = arg.trim();
        if message.is_empty() {
            return Err("magit.describe requires a non-empty description".to_owned());
        }
        return Ok(Some(MagitAction::JjDescribe {
            message: message.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.jj.new ") {
        let msg = arg.trim();
        return Ok(Some(MagitAction::JjNew {
            message: if msg.is_empty() {
                None
            } else {
                Some(msg.to_owned())
            },
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.stash.push ") {
        let msg = arg.trim();
        return Ok(Some(MagitAction::StashPush {
            message: if msg.is_empty() {
                None
            } else {
                Some(msg.to_owned())
            },
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.branch.create ") {
        let name = arg.trim();
        if name.is_empty() {
            return Err("magit.branch.create requires a branch name".to_owned());
        }
        return Ok(Some(MagitAction::CheckoutBranch {
            name: name.to_owned(),
            create: true,
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.branch.delete ") {
        let name = arg.trim();
        if name.is_empty() {
            return Err("magit.branch.delete requires a branch name".to_owned());
        }
        return Ok(Some(MagitAction::DeleteBranch {
            name: name.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.branch ") {
        let name = arg.trim();
        if name.is_empty() {
            return Err("magit.branch requires a branch name".to_owned());
        }
        return Ok(Some(MagitAction::CheckoutBranch {
            name: name.to_owned(),
            create: false,
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.cherry-pick ") {
        let rev = arg.trim();
        if rev.is_empty() {
            return Err("magit.cherry-pick requires a revision".to_owned());
        }
        return Ok(Some(MagitAction::CherryPick {
            rev: rev.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.revert ") {
        let rev = arg.trim();
        if rev.is_empty() {
            return Err("magit.revert requires a revision".to_owned());
        }
        return Ok(Some(MagitAction::RevertCommit {
            rev: rev.to_owned(),
        }));
    }
    if let Some(arg) = cmd.strip_prefix("magit.audit.record ") {
        let mut parts = arg.splitn(4, '|');
        let hunk_id = parts.next().unwrap_or("").trim();
        let actor = parts.next().unwrap_or("human").trim();
        let verdict = parts.next().unwrap_or("audited").trim();
        let rationale = parts.next().unwrap_or("").trim();
        if hunk_id.is_empty() {
            return Err("magit.audit.record requires hunk_id|actor|verdict|rationale".to_owned());
        }
        return Ok(Some(MagitAction::RecordJointAudit {
            hunk_id: hunk_id.to_owned(),
            actor: actor.to_owned(),
            verdict: verdict.to_owned(),
            rationale: rationale.to_owned(),
        }));
    }
    Err(format!("unsupported magit command: {cmd:?}"))
}

fn receipt_to_trajectory_event(
    receipt: &MagitActionReceipt,
    snap: &MagitSnapshot,
) -> GraphTrajectoryEvent {
    let mut metadata = HashMap::new();
    metadata.insert("backend".to_owned(), snap.backend.as_str().to_owned());
    metadata.insert("branch".to_owned(), snap.branch.clone());
    metadata.insert(
        "aitia_evidence_id".to_owned(),
        receipt.aitia_evidence_id.clone(),
    );
    metadata.insert(
        "aitia_decision".to_owned(),
        snap.aitia.aitia_decision.clone(),
    );
    metadata.insert("summary".to_owned(), receipt.summary.clone());
    if let Some(ref cid) = snap.change_id {
        metadata.insert("change_id".to_owned(), cid.as_str().to_owned());
    }
    if let Some(ref op) = receipt.op_id_after {
        metadata.insert("op_id_after".to_owned(), op.as_str().to_owned());
    }
    GraphTrajectoryEvent {
        event_id: receipt.aitia_evidence_id.clone(),
        run_id: format!("magit:{}", snap.repo_name),
        node_id: receipt.action.clone(),
        node_kind: "magit.vcs".to_owned(),
        status: if receipt.ok {
            "completed".to_owned()
        } else {
            "failed".to_owned()
        },
        node_receipt: None,
        metadata,
    }
}

fn project_overlay_record(
    overlay: &AitiaFormalOverlayProjection,
    repo_root: &str,
) -> GraphRecordView {
    let mut of = HashMap::new();
    of.insert("tool".to_owned(), overlay.tool.clone());
    of.insert(
        "passed".to_owned(),
        if overlay.passed { "true" } else { "false" }.to_owned(),
    );
    of.insert(
        "countermodel_verified".to_owned(),
        if overlay.countermodel_verified {
            "true"
        } else {
            "false"
        }
        .to_owned(),
    );
    of.insert(
        "proof_clean".to_owned(),
        if overlay.proof_clean { "true" } else { "false" }.to_owned(),
    );
    of.insert(
        "stored_in_sidecar_vault".to_owned(),
        if overlay.stored_in_sidecar_vault {
            "true"
        } else {
            "false"
        }
        .to_owned(),
    );
    of.insert("item_count".to_owned(), overlay.item_count.to_string());
    if let Some(ref ext) = overlay.extends_model {
        of.insert("extends_model".to_owned(), ext.clone());
    }
    if let Some(ref p) = overlay.archived_overlay_path {
        of.insert("archived_overlay_path".to_owned(), p.clone());
    }
    if let Some(ref p) = overlay.archived_receipt_path {
        of.insert("archived_receipt_path".to_owned(), p.clone());
    }
    GraphRecordView {
        record_id: format!("aitia-formal-overlay:{}", overlay.tool),
        surface: GraphVisibleSurface::Evidence as i32,
        kind: "aitia.formal.overlay".to_owned(),
        title: format!(
            "Formal MR05 Overlay ({}) · {}",
            overlay.tool,
            if overlay.passed { "PASSED" } else { "FAILED" }
        ),
        summary: overlay
            .archived_overlay_path
            .clone()
            .unwrap_or_else(|| "ephemeral sidecar vault".to_owned()),
        source_path: Some(repo_root.to_owned()),
        start_line: None,
        end_line: None,
        facets: of,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marlin_agent_gitops::magit::AitiaFormalModelRecommendation;

    #[test]
    fn project_progressive_review_emits_zero_authority_disposition_and_vault_rows() {
        let review = AitiaProgressiveReviewSnapshot {
            choice_status: "unique-deduction".to_owned(),
            world_count: 1,
            admissible: false,
            action_authorized: false,
            human_disposition_required: true,
            shared_prefix_cas_hit: true,
            plan_format: "sexp-aitia-review-plan".to_owned(),
            plan_valid: true,
            plan_acyclic: true,
            context_digest: "sha256:ctx-magit-1".to_owned(),
            open_obligations: vec!["SWE-083".to_owned(), "SWE-087".to_owned()],
            active_rule_packs: vec!["de-g2-core".to_owned()],
            findings: Vec::new(),
            quint_overlay: Some(AitiaFormalOverlayProjection {
                tool: "quint".to_owned(),
                passed: true,
                countermodel_verified: true,
                proof_clean: true,
                stored_in_sidecar_vault: true,
                extends_model: Some("oauth-token-lifecycle".to_owned()),
                archived_overlay_path: Some("/tmp/vault/x.qnt".to_owned()),
                archived_receipt_path: Some("/tmp/vault/x.qnt.receipt.json".to_owned()),
                item_count: 2,
            }),
            lean_overlay: Some(AitiaFormalOverlayProjection {
                tool: "lean4".to_owned(),
                passed: true,
                countermodel_verified: true,
                proof_clean: true,
                stored_in_sidecar_vault: true,
                extends_model: Some("ControlPlaneAuthAndGovernanceConfinement".to_owned()),
                archived_overlay_path: Some("/tmp/vault/y.lean".to_owned()),
                archived_receipt_path: Some("/tmp/vault/y.lean.receipt.json".to_owned()),
                item_count: 3,
            }),
            recommended_formal_models: vec![AitiaFormalModelRecommendation {
                model_id: "oauth-token-lifecycle".to_owned(),
                tool: "quint".to_owned(),
                library: "@deterministic-engineering/quint-models".to_owned(),
                rationale: "Token replay and epoch invariants".to_owned(),
            }],
        };

        let records = WorkspaceMagitBridge::project_progressive_review(&review, "/repo");
        assert_eq!(records.len(), 4);
        assert_eq!(records[0].kind, "aitia.review.step");
        assert_eq!(
            records[0]
                .facets
                .get("action_authorized")
                .map(String::as_str),
            Some("false")
        );
        assert_eq!(
            records[0]
                .facets
                .get("human_disposition_required")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            records[0]
                .facets
                .get("shared_prefix_cas_hit")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(records[1].kind, "aitia.formal.overlay");
        assert_eq!(
            records[1]
                .facets
                .get("archived_overlay_path")
                .map(String::as_str),
            Some("/tmp/vault/x.qnt")
        );
        assert_eq!(records[2].kind, "aitia.formal.overlay");
        assert_eq!(
            records[2]
                .facets
                .get("archived_overlay_path")
                .map(String::as_str),
            Some("/tmp/vault/y.lean")
        );
        assert_eq!(records[3].kind, "aitia.formal.recommendation");
    }

    #[tokio::test]
    async fn live_magit_aitia_review_command_projects_real_sidecar_step_without_mutating_git() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let aitia_bin = PathBuf::from(home).join(".local/bin/aitia");
        if !aitia_bin.is_file() {
            return;
        }
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).expect("mkdir");
        let run_git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .current_dir(&repo)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?} failed", args);
        };
        run_git(&["init", "-b", "main"]);
        run_git(&["config", "user.email", "test@example.com"]);
        run_git(&["config", "user.name", "Test"]);
        std::fs::write(
            repo.join("src/auth.rs"),
            "pub fn check(tok: Result<String, &'static str>) -> Result<String, &'static str> {\n    let s = tok?;\n    assert!(!s.is_empty());\n    Ok(s)\n}\n",
        )
        .expect("write baseline");
        run_git(&["add", "."]);
        run_git(&["commit", "-m", "baseline"]);
        // Inject unstaged silent fallback defect
        std::fs::write(
            repo.join("src/auth.rs"),
            "pub fn check(tok: Result<String, &'static str>) -> Result<String, &'static str> {\n    let s = tok.unwrap_or_default();\n    Ok(s)\n}\n",
        )
        .expect("write defect");

        let bridge = WorkspaceMagitBridge::new(repo.clone());
        let (receipt, snap, traj) = bridge
            .execute_ui_command("magit.aitia.review")
            .await
            .expect("magit.aitia.review must succeed")
            .expect("must recognize magit.aitia.review");
        assert!(receipt.ok);
        assert!(receipt.requires_gate);
        assert_eq!(traj.node_id, "magit.aitia.review");

        let review = bridge
            .latest_progressive_review()
            .expect("bridge must cache progressive review");
        assert!(!review.action_authorized);
        assert!(!review.admissible);
        assert!(review.human_disposition_required);
        assert!(review.world_count >= 1);
        assert!(!review.findings.is_empty());

        let projected = WorkspaceMagitBridge::project_snapshot_with_review(
            &snap,
            Some(&receipt),
            Some(&review),
        );
        assert!(projected.iter().any(|r| r.kind == "aitia.review.step"));
        assert!(
            projected
                .iter()
                .any(|r| r.kind == "aitia.formal.recommendation")
        );
    }

    #[tokio::test]
    async fn snapshot_fast_for_serves_warm_cache_submillisecond_and_honors_force_refresh() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo).expect("mkdir");
        let run_git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .current_dir(&repo)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?} failed", args);
        };
        run_git(&["init", "-b", "main"]);
        run_git(&["config", "user.email", "test@example.com"]);
        run_git(&["config", "user.name", "Test"]);
        std::fs::write(repo.join("README.md"), "# fast magit\n").expect("write");
        run_git(&["add", "."]);
        run_git(&["commit", "-m", "initial"]);

        let bridge = WorkspaceMagitBridge::new(repo.clone());
        let first = bridge
            .snapshot_fast_for(None, false)
            .await
            .expect("cold snapshot");
        assert_eq!(first.revision, 1);

        let t0 = std::time::Instant::now();
        let warm = bridge
            .snapshot_fast_for(None, false)
            .await
            .expect("warm cached snapshot");
        let warm_elapsed = t0.elapsed();
        assert_eq!(warm.revision, first.revision);
        assert!(
            warm_elapsed < Duration::from_millis(5),
            "expected sub-5ms cached hit, got {warm_elapsed:?}"
        );

        let forced = bridge
            .snapshot_fast_for(None, true)
            .await
            .expect("forced refresh");
        assert!(forced.revision > first.revision);
    }
}
