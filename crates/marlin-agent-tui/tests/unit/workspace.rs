// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::workspace::{
    WorkspaceObservation, WorkspaceQuery, WorkspaceStatus, WorkspaceView, parse_query,
};
use marlin_git_utils::{GitDiffScope, GitStatus};

#[test]
fn diff_selection_uses_json_path_and_explicit_scope() {
    let WorkspaceQuery::Diff(request) = parse_query(r#"/diff "file [1].txt""#).unwrap() else {
        panic!("diff")
    };
    assert_eq!(request.scope, GitDiffScope::Worktree);
    assert_eq!(request.path.unwrap().as_str(), "file [1].txt");
    let WorkspaceQuery::Diff(request) = parse_query(r#"/diff staged "name\nnext""#).unwrap() else {
        panic!("diff")
    };
    assert_eq!(request.scope, GitDiffScope::Index);
    assert_eq!(request.path.unwrap().as_str(), "name\nnext");
    for invalid in [
        r#"/diff "../private""#,
        r#"/diff "/absolute""#,
        "/diff file.txt",
        r#"/diff "file" trailing"#,
    ] {
        assert!(parse_query(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn workspace_view_replaces_previous_result_and_closes() {
    let mut workspace = WorkspaceStatus::default();
    for _ in 0..20 {
        workspace.show(WorkspaceView::from_observation(
            WorkspaceObservation::Status(GitStatus {
                entries: Vec::new(),
            }),
        ));
        let view = workspace.view().unwrap();
        assert_eq!(view.transcript.len(), 1);
        assert!(view.transcript[0].1.contains("Worktree clean"));
        assert_eq!(view.scroll_from_top, 0);
    }
    assert!(workspace.close_view());
    assert!(workspace.view().is_none());
    assert!(!workspace.close_view());
}

#[test]
fn cancelled_read_must_retire_before_another_read_starts() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let path = std::env::current_dir().unwrap();
    let mut workspace = WorkspaceStatus::default();
    assert!(workspace.start(&runtime, path.clone(), WorkspaceQuery::Status));
    assert!(workspace.cancel());
    assert!(!workspace.start(&runtime, path.clone(), WorkspaceQuery::Status));
    runtime.block_on(async { tokio::task::yield_now().await });
    assert!(workspace.start(&runtime, path, WorkspaceQuery::Status));
    assert!(workspace.cancel());
}
