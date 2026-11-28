// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Startup session, artifacts layout, and tool catalog binding.

use std::sync::Arc;

use super::{
    AgentId, App, JournalGateway, ProjectId, RuntimeStreamGateway, SessionBrowser, SessionId,
    SharedGateway, TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode,
    TursoOptimizationProfile, config, load_session,
};
#[cfg(feature = "linked-ui")]
use super::{MessageEvent, MessageSeverity, MessageSource};

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
pub(super) struct ToolCatalogRequest<'a> {
    pub(super) scheme_runtime: &'a gerbil_scheme::GerbilRuntime,
    pub(super) directory: &'a std::path::Path,
    pub(super) effective_session: Option<&'a config::SessionConfig>,
    pub(super) search_dsl_runner: marlin_agent_tooling::AsyncSearchDslSessionRunner,
    pub(super) runtime_handle: &'a tokio::runtime::Handle,
    pub(super) runtime_owner: &'a marlin_agent_runtime::TokioAgentRuntime,
}

#[cfg(feature = "linked-ui")]
pub(super) struct BoundToolCatalog {
    pub(super) catalog: Option<Arc<marlin_agent_runtime::ModelToolCatalog>>,
    pub(super) search_index_handle: marlin_agent_tooling::SearchGatewayIndexHandle,
    pub(super) grpc_server: Option<marlin_agent_tooling::LoopbackSearchGrpcServer>,
    pub(super) active_search_session:
        Arc<std::sync::RwLock<marlin_agent_search_engine::SearchDslSessionId>>,
}

#[cfg(feature = "linked-ui")]
pub(super) fn search_session_id_for(
    raw: &str,
) -> marlin_agent_search_engine::SearchDslSessionId {
    let sanitized: String = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    let candidate = if sanitized.is_empty() {
        "tui-session".to_owned()
    } else {
        format!("tui-{sanitized}")
    };
    marlin_agent_search_engine::SearchDslSessionId::new(candidate).unwrap_or_else(|_| {
        marlin_agent_search_engine::SearchDslSessionId::new("tui-default")
            .expect("static search session ID")
    })
}

#[cfg(feature = "linked-ui")]
pub(super) fn sync_active_search_session(
    cell: &Arc<std::sync::RwLock<marlin_agent_search_engine::SearchDslSessionId>>,
    raw_session_id: &str,
) {
    let next = search_session_id_for(raw_session_id);
    if let Ok(mut guard) = cell.write() {
        *guard = next;
    }
}

#[cfg(feature = "linked-ui")]
pub(super) fn build_tool_catalog(
    req: ToolCatalogRequest<'_>,
) -> Result<BoundToolCatalog, Box<dyn std::error::Error>> {
    let search_index_handle =
        marlin_agent_tooling::SearchGatewayIndexHandle::lazy(req.directory.to_path_buf());
    let active_search_session = Arc::new(std::sync::RwLock::new(search_session_id_for("default")));
    if req.effective_session.is_none() {
        return Ok(BoundToolCatalog {
            catalog: None,
            search_index_handle,
            grpc_server: None,
            active_search_session,
        });
    }
    let declarations = super::super::linked_ui::tooling_declarations(req.scheme_runtime)?;
    let search_decls: Vec<_> = declarations
        .iter()
        .filter(|decl| decl.capability() == marlin_agent_runtime::ModelToolCapability::Search)
        .cloned()
        .collect();
    let service = marlin_agent_tooling::SearchGatewayService::with_lazy_async_runner(
        search_index_handle.clone(),
        marlin_agent_tooling::tool_schema_entries_from_declarations(&search_decls),
        req.search_dsl_runner,
        req.runtime_owner.context(),
    );
    let server = marlin_agent_tooling::LoopbackSearchGrpcServer::spawn_on(
        req.runtime_handle,
        service,
        req.runtime_owner,
    )
    .map_err(|error| format!("cannot start loopback search gRPC server: {error}"))?;
    let catalog = marlin_agent_tooling::catalog_from_declarations_with_grpc_search(
        declarations,
        req.directory.to_path_buf(),
        server.client(),
        Arc::clone(&active_search_session),
    )
    .map_err(|error| format!("cannot bind Scheme tool catalog: {error:?}"))?;
    Ok(BoundToolCatalog {
        catalog: Some(Arc::new(catalog)),
        search_index_handle,
        grpc_server: Some(server),
        active_search_session,
    })
}

#[cfg(feature = "linked-ui")]
pub(super) fn bind_artifact_layout(
    app: &mut App,
    bound_tools: BoundToolCatalog,
    artifact_layout: Option<marlin_agent_context::artifacts::WorkspaceArtifactLayout>,
) {
    app.search_index_handle = bound_tools.search_index_handle;
    app._search_grpc_server = bound_tools.grpc_server;
    app.active_search_session = bound_tools.active_search_session;
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
    #[cfg(feature = "linked-ui")]
    sync_active_search_session(&app.active_search_session, session_id.as_str());
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

#[cfg(all(test, feature = "linked-ui"))]
#[path = "../../tests/unit/startup_grpc_e2e.rs"]
mod grpc_e2e_tests;
