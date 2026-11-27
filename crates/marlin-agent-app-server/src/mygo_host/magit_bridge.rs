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
};

use marlin_agent_gitops::magit::{
    AitiaMarlinGrpcClient, MagitAction, MagitActionReceipt, MagitDiffHunk, MagitDiffLineKind,
    MagitEngine, MagitFileEntry, MagitJointAuditRecord, MagitLogEntry, MagitSnapshot,
    build_single_hunk_patch,
};

use crate::wire::marlin::projection::v1::{
    FileDiffEntry, GraphRecordView, GraphTrajectoryEvent, GraphVisibleSurface,
    WorkspaceDiffSnapshot,
};

const MAX_HUNK_PREVIEW_LINES: usize = 48;

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
    last_receipt: Arc<RwLock<Option<MagitActionReceipt>>>,
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
            last_receipt: Arc::new(RwLock::new(None)),
        }
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
        self.last_receipt.read().ok().and_then(|guard| guard.clone())
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
        let engine = MagitEngine::discover(&self.workspace_root)
            .await
            .map_err(|e| e.to_string())?;
        let rev = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let aitia_report = self
            .aitia_client
            .audit_diff_defense(&self.workspace_root, "HEAD")
            .await;
        let joints = self.current_joint_audits();
        let snap = engine
            .snapshot_with_evidence(rev, &aitia_report, &joints)
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(mut guard) = self.cached_snapshot.write() {
            *guard = Some(snap.clone());
        }
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
            rf.insert("ok".to_owned(), if r.ok { "true" } else { "false" }.to_owned());
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
