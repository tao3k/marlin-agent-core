// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{fs, path::Path, process::Command};

use marlin_agent_app_server::{
    ui_grpc::MyGoBridgeHost,
    wire::marlin::{
        projection::v1::{
            QueryProjectGraphRequest, SecretVisibility, WatchWorkspaceDiffRequest,
            projection_query_service_client::ProjectionQueryServiceClient,
        },
        ui::v1::{
            DispatchUiEventRequest, dispatch_ui_event_request,
            ui_workbench_service_client::UiWorkbenchServiceClient,
        },
    },
};
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use tonic::transport::Endpoint;

use super::support::auth_req;

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git must execute");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::test]
async fn magit_grpc_bridge_projects_hunks_and_executes_stage_commit_with_aitia_receipts() {
    let dir = tempfile::tempdir().expect("tempdir");
    run_git(dir.path(), &["init", "-b", "main"]);
    run_git(dir.path(), &["config", "user.name", "Marlin Test"]);
    run_git(dir.path(), &["config", "user.email", "test@marlin.local"]);
    fs::write(dir.path().join("auth.rs"), "pub fn check() -> bool { true }\n").unwrap();
    run_git(dir.path(), &["add", "auth.rs"]);
    run_git(dir.path(), &["commit", "-m", "feat: initial auth check"]);

    // Modify auth.rs with a security-sensitive diff.
    fs::write(
        dir.path().join("auth.rs"),
        "pub fn check() -> bool {\n    let unsafe_bypass = false;\n    !unsafe_bypass\n}\n",
    )
    .unwrap();

    let host = MyGoBridgeHost::new_default_trust(|meta| {
        MyGoBridgeHost::check_bearer_header(meta, Some("Bearer test-token"))
    })
    .with_magit_workspace(dir.path().to_path_buf());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let _ = host.serve_standalone_on(listener).await;
    });

    let channel = Endpoint::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut proj = ProjectionQueryServiceClient::new(channel.clone());
    let mut ui = UiWorkbenchServiceClient::new(channel);

    // 1. Query live Magit snapshot over QueryProjectGraph("magit:snapshot").
    let resp = proj
        .query_project_graph(auth_req(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            query: "magit:snapshot".to_owned(),
            surfaces: vec![],
            secret_visibility: SecretVisibility::Unspecified as i32,
            page: None,
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(
        resp.records.iter().any(|r| r.kind == "magit.head"
            && r.facets.get("branch").map(String::as_str) == Some("main")),
        "expected magit.head record on main"
    );
    assert!(
        resp.records.iter().any(|r| r.kind == "aitia.evidence"),
        "expected Lambda Aitia evidence record"
    );
    assert!(
        resp.records.iter().any(|r| r.kind == "magit.hunk"
            && r.source_path.as_deref() == Some("auth.rs")
            && r.facets.contains_key("evidence_chain")),
        "expected magit.hunk record with explicit evidence_chain facet for auth.rs"
    );

    // 1b. Record a Human-AI joint audit flag on that hunk and verify evidence chain elevates risk to high!
    let hunk_id = resp
        .records
        .iter()
        .find(|r| r.kind == "magit.hunk" && r.source_path.as_deref() == Some("auth.rs"))
        .unwrap()
        .record_id
        .clone();
    let flag_resp = ui
        .dispatch_ui_event(auth_req(DispatchUiEventRequest {
            session_id: "default".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(format!(
                "magit.audit.record {hunk_id}|joint|flagged|Needs token boundary review"
            ))),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(flag_resp.admitted, "magit.audit.record must be admitted");

    let flagged_snap = proj
        .query_project_graph(auth_req(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            query: "magit:snapshot".to_owned(),
            surfaces: vec![],
            secret_visibility: SecretVisibility::Unspecified as i32,
            page: None,
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(
        flagged_snap.records.iter().any(|r| r.kind == "magit.hunk"
            && r.record_id == hunk_id
            && r.facets.get("risk").map(String::as_str) == Some("high")
            && r.facets
                .get("evidence_chain")
                .is_some_and(|e| e.contains("Needs token boundary review"))),
        "expected joint audit flag to elevate hunk risk to high with cited rationale"
    );

    // 2. WatchWorkspaceDiff must project live modified files instead of hardcoded stub.
    let mut diff_stream = proj
        .watch_workspace_diff(auth_req(WatchWorkspaceDiffRequest {
            project_id: "default".to_owned(),
            worktree_id: None,
        }))
        .await
        .unwrap()
        .into_inner();
    let diff_msg = diff_stream.next().await.unwrap().unwrap();
    let diff_snap = diff_msg.snapshot.unwrap();
    assert_eq!(diff_snap.branch, "main");
    assert!(diff_snap.files.iter().any(|f| f.path == "auth.rs"));

    // 3. Stage & commit through governance-gated DispatchUiEvent(RecordCommand("magit.*")).
    let stage_resp = ui
        .dispatch_ui_event(auth_req(DispatchUiEventRequest {
            session_id: "default".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "magit.stage auth.rs".to_owned(),
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(stage_resp.admitted, "magit.stage must be admitted");

    let commit_resp = ui
        .dispatch_ui_event(auth_req(DispatchUiEventRequest {
            session_id: "default".to_owned(),
            expected_generation: 0,
            event: Some(dispatch_ui_event_request::Event::RecordCommand(
                "magit.commit fix(auth): harden check".to_owned(),
            )),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(commit_resp.admitted, "magit.commit must be admitted");

    // 4. Verify fresh snapshot has clean working tree, new log entry, and magit.receipt.
    let post = proj
        .query_project_graph(auth_req(QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            query: "magit:snapshot".to_owned(),
            surfaces: vec![],
            secret_visibility: SecretVisibility::Unspecified as i32,
            page: None,
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(
        post.records
            .iter()
            .any(|r| r.kind == "magit.receipt" && r.facets.get("ok").map(String::as_str) == Some("true")),
        "expected magit.receipt with ok=true"
    );
    assert!(
        post.records
            .iter()
            .any(|r| r.kind == "magit.log" && r.title.contains("harden check")),
        "expected new commit in magit.log"
    );
    assert!(
        !post.records.iter().any(|r| r.kind == "magit.file"),
        "working tree should be clean after commit"
    );

    server.abort();
}
