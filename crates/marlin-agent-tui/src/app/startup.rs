// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Startup session, artifacts layout, and tool catalog binding.

use std::sync::Arc;

use super::{
    AgentId, App, JournalGateway, MessageEvent, MessageSeverity, MessageSource, ProjectId,
    RuntimeStreamGateway, SessionBrowser, SessionId, SharedGateway, TursoAgentStorage,
    TursoAgentStorageConfig, TursoBatchTransactionMode, TursoOptimizationProfile, config,
    load_session,
};

#[cfg(feature = "linked-ui")]
pub(super) fn resolve_effective_session(
    explicit: Option<config::SessionConfig>,
    artifact_layout: Option<&marlin_agent_context::artifacts::WorkspaceArtifactLayout>,
) -> Option<config::SessionConfig> {
    explicit.or_else(|| {
        artifact_layout.map(|layout| config::SessionConfig {
            path: layout.session_db_path.clone(),
            project: layout.project.clone(),
            session: layout.session.clone(),
            agent: layout.agent.clone(),
        })
    })
}

#[cfg(feature = "linked-ui")]
pub(super) fn build_tool_catalog(
    scheme_runtime: &gerbil_scheme::GerbilRuntime,
    directory: &std::path::Path,
    effective_session: Option<&config::SessionConfig>,
    search_tool_slot: &marlin_agent_tooling::SharedSearchIndexSlot,
) -> Result<Option<Arc<marlin_agent_runtime::ModelToolCatalog>>, Box<dyn std::error::Error>> {
    if effective_session.is_none() {
        return Ok(None);
    }
    let declarations = super::super::linked_ui::tooling_declarations(scheme_runtime)?;
    let catalog = marlin_agent_tooling::catalog_from_declarations_with_search(
        declarations,
        directory.to_path_buf(),
        search_tool_slot.clone(),
    )
    .map_err(|error| format!("cannot bind Scheme tool catalog: {error:?}"))?;
    Ok(Some(Arc::new(catalog)))
}

#[cfg(feature = "linked-ui")]
pub(super) fn bind_artifact_layout(
    app: &mut App,
    search_tool_slot: marlin_agent_tooling::SharedSearchIndexSlot,
    artifact_layout: Option<marlin_agent_context::artifacts::WorkspaceArtifactLayout>,
) {
    app.search_tool_slot = search_tool_slot;
    if let Some(layout) = artifact_layout {
        app.search_cache_dir = Some(layout.search_dir.clone());
        let detail = format!(
            "root={} · session={} · agent={} · search={}",
            layout.root.display(),
            layout.session_db_path.display(),
            layout.agent,
            layout.search_dir.display(),
        );
        app.publish_message(MessageEvent {
            source: MessageSource::Workspace,
            severity: MessageSeverity::Info,
            summary: "Workspace Artifacts layout bound",
            detail: &detail,
        });
    }
}

pub(super) fn open_session_gateway(
    app: &mut App,
    runtime: &tokio::runtime::Runtime,
    gateway: RuntimeStreamGateway,
    effective_session: Option<config::SessionConfig>,
    selected_session: Option<SessionId>,
    tool_catalog: Option<&Arc<marlin_agent_runtime::ModelToolCatalog>>,
) -> Result<SharedGateway, Box<dyn std::error::Error>> {
    let Some(session) = effective_session else {
        return Ok(SharedGateway::new(gateway));
    };
    let session_id = match selected_session {
        Some(id) => id,
        None => SessionId::new(session.session)?,
    };
    let browser_owner = app.runtime_owner.clone();
    let (journal, turns, browser) = runtime.block_on(async {
        let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: session.path,
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: TursoBatchTransactionMode::Immediate,
        })
        .await?;
        let project = ProjectId::new(session.project)?;
        let agent = AgentId::new(session.agent)?;
        let browser = SessionBrowser::with_runtime(
            storage.clone(),
            project.clone(),
            agent.clone(),
            browser_owner,
        );
        let (journal, turns) = load_session(storage, project, session_id.clone(), agent).await?;
        Ok::<_, Box<dyn std::error::Error>>((journal, turns, browser))
    })?;
    app.session_browser = Some(browser);
    app.current_session = Some(session_id.clone());
    app.transcript
        .push(("Session".into(), session_id.as_str().to_owned()));
    app.restore(turns);
    let bound = JournalGateway::new(gateway, journal).with_tool_offer(tool_catalog.map_or_else(
        || marlin_agent_runtime::ModelToolCatalog::new().offer(),
        |catalog| catalog.offer(),
    ));
    app.tool_review = bound.tool_review_handle();
    Ok(SharedGateway::new(bound))
}
