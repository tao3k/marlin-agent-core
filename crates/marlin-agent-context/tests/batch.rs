// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_context::{
    CONTEXT_BATCH_MAX_ROWS, ContextAdmissionError, ContextBundleId, ContextDestinationId,
    ContextOrganizationId, ContextScopeBatch, ContextScopeEpoch, ContextScopeKey, ContextScopeRow,
    ContextSourceCutId, ContextTaskId,
};
use marlin_agent_protocol::{
    ProjectRuntimeAgentId, ProjectRuntimeProjectId, ProjectRuntimeWorktreeId,
};

#[test]
fn admission_bounds_batch_rows_and_utf8_before_vm_entry() {
    let bundle = ContextBundleId::new("bundle");
    let organization = ContextOrganizationId::new("org");
    let project = ProjectRuntimeProjectId::new("project");
    let worktree = ProjectRuntimeWorktreeId::new("worktree");
    let cut = ContextSourceCutId::new("cut");
    let actor = ProjectRuntimeAgentId::new("actor");
    let task = ContextTaskId::new("🦦".repeat(256));
    let oversized = ContextTaskId::new("界".repeat(257));
    let destination = ContextDestinationId::new("provider");
    let key = || ContextScopeKey {
        bundle: &bundle,
        organization: &organization,
        project: &project,
        worktree: &worktree,
        source_cut: &cut,
        epoch: ContextScopeEpoch(1),
    };
    let row = |task| ContextScopeRow {
        actor: &actor,
        task,
        destination: &destination,
        session_turn: None,
    };

    let rows = [row(&task)];
    let batch = ContextScopeBatch::new(key(), &rows).expect("a partial batch can dispatch");
    assert_eq!(batch.rows().len(), 1);
    assert!(matches!(
        ContextScopeBatch::new(key(), &[row(&oversized)]),
        Err(ContextAdmissionError::InvalidIdentifier),
    ));

    let rows: Vec<_> = (0..CONTEXT_BATCH_MAX_ROWS).map(|_| row(&task)).collect();
    assert!(matches!(
        ContextScopeBatch::new(key(), &rows),
        Err(ContextAdmissionError::TextBudget),
    ));
    let rows: Vec<_> = (0..=CONTEXT_BATCH_MAX_ROWS).map(|_| row(&task)).collect();
    assert!(matches!(
        ContextScopeBatch::new(key(), &rows),
        Err(ContextAdmissionError::RowCount),
    ));
}

#[test]
fn row_text_budget_counts_utf8_and_optional_session_without_numeric_turn() {
    use marlin_agent_context::{ContextSession, ContextTurn, ContextTurnIndex};
    use marlin_agent_protocol::ProjectRuntimeSessionId;

    let mut turn = ContextTurn {
        actor: ProjectRuntimeAgentId::new("代理"),
        task: ContextTaskId::new("任务🦦"),
        destination: ContextDestinationId::new("panel"),
        session: None,
    };
    assert_eq!(turn.text_bytes(), "代理任务🦦panel".len());
    turn.session = Some(ContextSession {
        session: ProjectRuntimeSessionId::new("会话🦦"),
        turn: ContextTurnIndex(u64::MAX),
    });
    assert_eq!(turn.text_bytes(), "代理任务🦦panel会话🦦".len());
}
