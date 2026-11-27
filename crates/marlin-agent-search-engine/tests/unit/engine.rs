// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::time::{SystemTime, UNIX_EPOCH};

use marlin_agent_search_engine::{
    WorkspaceIndexConfig, WorkspaceSearchQuery, open_or_build_workspace_index_safe,
};

#[test]
fn open_or_build_workspace_index_safe_preserves_base_trigrams_across_disk_edits() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "marlin-safe-mmap-regression-{}-{nonce}",
        std::process::id()
    ));
    let src_dir = base.join("src");
    std::fs::create_dir_all(&src_dir).expect("create temp src dir");

    // Seed untouched base files and one file that will be edited after snapshot creation.
    for i in 0..8 {
        std::fs::write(
            src_dir.join(format!("mod_{i}.rs")),
            format!("pub struct StableImpl{i};\nimpl ModelGateway for StableImpl{i} {{}}\n"),
        )
        .expect("write base file");
    }
    let mutable_file = src_dir.join("mutable.rs");
    std::fs::write(&mutable_file, "pub fn initial_marker() {}\n").expect("write mutable file");

    let snapshot_path = base.join("cache").join("workspace.asp-index.bin");

    // Build initial snapshot on disk.
    let first = open_or_build_workspace_index_safe(&base, &snapshot_path, WorkspaceIndexConfig::default());
    assert_eq!(first.file_count(), 9);
    drop(first);

    // Modify one file on disk so the next open triggers mmap load + incremental sync + snapshot rewrite.
    std::fs::write(
        &mutable_file,
        "pub fn updated_marker() {}\nimpl ModelGateway for MutableImpl {}\n",
    )
    .expect("update mutable file");

    // Reopen via safe path: must compact before atomic temp+rename so base files are never lost.
    let reopened =
        open_or_build_workspace_index_safe(&base, &snapshot_path, WorkspaceIndexConfig::default());
    assert_eq!(reopened.file_count(), 9);

    let hits = reopened.query(&WorkspaceSearchQuery {
        pattern: "re:impl\\s+ModelGateway\\s+for",
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 32,
        context_radius: 0,
    });
    assert_eq!(
        hits.hits.len(),
        9,
        "all 8 untouched base files + 1 edited file must match after safe snapshot sync"
    );

    // Reopen a third time from the newly written atomic snapshot to verify on-disk integrity.
    drop(reopened);
    let third =
        open_or_build_workspace_index_safe(&base, &snapshot_path, WorkspaceIndexConfig::default());
    let hits_third = third.query(&WorkspaceSearchQuery {
        pattern: "ModelGateway",
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 32,
        context_radius: 0,
    });
    assert_eq!(hits_third.hits.len(), 9);

    let _ = std::fs::remove_dir_all(&base);
}
