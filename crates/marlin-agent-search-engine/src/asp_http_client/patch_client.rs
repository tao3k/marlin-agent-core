// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Closed-loop HTTP AST patch client (`POST /v1/ast/patch`):
//! connects Marlin's `:gerbil-ascent` Search DSL `entity_locator` output directly
//! to ASP's native-compiler lossless span splice + Frontier 2 `ast-verify`
//! calibration + immediate Open EDB refresh.

use std::fs;
use std::path::Path;

use agent_semantic_search::{
    WorkspaceFileDelta, WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex,
};
use serde::{Deserialize, Serialize};

use crate::SearchPlanProjectionError;

use super::domain_types::{AspForwardingEndpoint, AspLanguageId, AspProviderId};
use super::transport::http_loopback_request;

/// Typed structural AST edit operation forwarded to ASP `POST /v1/ast/patch`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum AspHttpAstPatchOp {
    ReplaceItem { target: String, snippet: String },
    AppendToBlock { target: String, snippet: String },
    InsertBeforeNode { target: String, snippet: String },
    InsertAfterNode { target: String, snippet: String },
    RemoveNode { target: String },
    InsertImport { import_statement: String },
}

/// Request submitting a locator-anchored structural edit through ASP's gateway.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AspHttpAstPatchRequest<'a> {
    pub workspace_root: &'a Path,
    pub path: &'a str,
    pub language_override: Option<&'a AspLanguageId>,
    pub expected_preimage_hash: Option<&'a str>,
    pub operation: AspHttpAstPatchOp,
    pub dry_run: bool,
    pub auto_calibrate: bool,
}

/// Outcome status returned by ASP `POST /v1/ast/patch`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AspHttpAstPatchStatus {
    Verified,
    Applied,
    CalibratedAndApplied,
    RejectedFailClosed,
}

/// Receipt returned after executing a closed-loop AST patch through ASP HTTP.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AspHttpAstPatchReceipt {
    pub status: AspHttpAstPatchStatus,
    pub path: String,
    pub language_id: AspLanguageId,
    pub provider_id: AspProviderId,
    pub preimage_hash: String,
    #[serde(default)]
    pub postimage_hash: Option<String>,
    #[serde(default)]
    pub patched_source_utf8: Option<String>,
    #[serde(default)]
    pub unified_diff: Option<String>,
    pub reparse_verified: bool,
    #[serde(default)]
    pub open_edb_batch: Option<WorkspaceOpenEdbIngestBatch>,
    #[serde(skip)]
    pub synced_entities: usize,
    #[serde(skip)]
    pub synced_edges: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstPatchRequest<'a> {
    path: &'a str,
    source_utf8: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_override: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_preimage_hash: Option<&'a str>,
    operation: &'a AspHttpAstPatchOp,
    dry_run: bool,
    auto_calibrate: bool,
}

fn atomic_write_file(target: &Path, bytes: &[u8]) -> Result<(), SearchPlanProjectionError> {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    static PATCH_TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ast-patch");
    let seq = PATCH_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!(
        ".{file_name}.{}.{seq}.marlin-ast-tmp",
        std::process::id()
    ));
    let write_res = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_res {
        let _ = fs::remove_file(&tmp);
        return Err(SearchPlanProjectionError::Native(format!(
            "write temp patch: {e}"
        )));
    }
    fs::rename(&tmp, target).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        SearchPlanProjectionError::Native(format!("commit atomic patch: {e}"))
    })?;
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

fn normalize_batch_language(
    batch: &mut WorkspaceOpenEdbIngestBatch,
    aliases: &std::collections::BTreeMap<String, String>,
) {
    for ent in &mut batch.entities {
        if let Some(canonical) = aliases.get(ent.language.as_str()) {
            ent.language.clone_from(canonical);
        }
    }
    if let Some(lang) = batch.language_id.as_deref()
        && let Some(canonical) = aliases.get(lang)
    {
        batch.language_id = Some(canonical.clone());
    }
}

fn commit_and_sync_patched_file(
    endpoint: &AspForwardingEndpoint,
    abs_path: &Path,
    rel_path: &str,
    index: &mut WorkspaceSearchIndex,
    receipt: &mut AspHttpAstPatchReceipt,
) -> Result<(), SearchPlanProjectionError> {
    let Some(patched) = receipt.patched_source_utf8.as_deref() else {
        return Ok(());
    };
    atomic_write_file(abs_path, patched.as_bytes())?;
    let _ = index.apply_file_delta(WorkspaceFileDelta {
        upserts: vec![(rel_path.to_owned(), patched.as_bytes().to_vec())],
        deletes: Vec::new(),
    });
    let Some(mut batch) = receipt.open_edb_batch.clone() else {
        return Ok(());
    };
    let catalog = super::transport::fetch_asp_provider_catalog(endpoint)?;
    let aliases = super::sync::provider_language_aliases(&catalog);
    normalize_batch_language(&mut batch, &aliases);
    let ingest = index.ingest_open_edb_facts(batch);
    receipt.synced_entities = ingest.ingested_entities;
    receipt.synced_edges = ingest.ingested_edges;
    Ok(())
}

/// Execute a closed-loop lossless AST patch over ASP `POST /v1/ast/patch`,
/// atomically commit the verified file when `!dry_run`, and synchronously
/// refresh Marlin's live [`WorkspaceSearchIndex`] + Open EDB entities & edges.
pub fn apply_ast_patch_via_asp_http(
    endpoint: &AspForwardingEndpoint,
    index: &mut WorkspaceSearchIndex,
    request: &AspHttpAstPatchRequest<'_>,
) -> Result<AspHttpAstPatchReceipt, SearchPlanProjectionError> {
    let abs_path = request.workspace_root.join(request.path);
    let source_utf8 = fs::read_to_string(&abs_path).map_err(|e| {
        SearchPlanProjectionError::Native(format!("read {}: {e}", abs_path.display()))
    })?;
    let wire = WireAstPatchRequest {
        path: request.path,
        source_utf8,
        language_override: request.language_override.map(AspLanguageId::as_str),
        expected_preimage_hash: request.expected_preimage_hash,
        operation: &request.operation,
        dry_run: request.dry_run,
        auto_calibrate: request.auto_calibrate,
    };
    let encoded = serde_json::to_vec(&wire)
        .map_err(|e| SearchPlanProjectionError::Native(format!("encode ast-patch request: {e}")))?;
    let (status, resp_body) =
        http_loopback_request(endpoint, "POST", "/v1/ast/patch", Some(&encoded))?;
    if status != 200 {
        return Err(SearchPlanProjectionError::Native(format!(
            "ASP POST /v1/ast/patch returned HTTP {status}: {}",
            String::from_utf8_lossy(&resp_body)
        )));
    }
    let mut receipt: AspHttpAstPatchReceipt = serde_json::from_slice(&resp_body).map_err(|e| {
        SearchPlanProjectionError::Native(format!("decode ast-patch response: {e}"))
    })?;

    let should_commit = !request.dry_run
        && matches!(
            receipt.status,
            AspHttpAstPatchStatus::Applied | AspHttpAstPatchStatus::CalibratedAndApplied
        );
    if should_commit {
        commit_and_sync_patched_file(endpoint, &abs_path, request.path, index, &mut receipt)?;
    }
    Ok(receipt)
}
