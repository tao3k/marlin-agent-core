// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit & stability tests for `marlin_agent_app_server::mygo_host`.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use marlin_agent_app_server::{
    SessionOpen,
    mygo_host::{
        DeterministicLocalGateway, MAX_RESIDENT_SESSIONS, ResidentSessionTable,
        config::{
            HostModelConfig, HotSwappableModelGateway, ParsedHelmQuery, RouteEntry,
            WorkspaceHelmSearchBridge,
        },
        lookup_host_session, run_ui_command_router_loop,
    },
    ui_grpc::{DispatchedUiCommand, MyGoBridgeHost},
    wire::marlin::projection::v1::{GraphVisibleSurface, QueryProjectGraphRequest},
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayCompletionResponse, ModelGatewayFuture,
    ModelGatewayRequest, ModelGatewayResult,
};
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId};
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamingGateway};
use tokio::sync::Mutex;

mod stability_tests {
    use super::{
        AgentId, Arc, BTreeMap, DeterministicLocalGateway, DispatchedUiCommand, Duration,
        InMemoryAgentStorage, MAX_RESIDENT_SESSIONS, ModelEndpoint, Mutex, MyGoBridgeHost,
        ProjectId, ResidentSessionTable, SessionOpen, lookup_host_session,
        run_ui_command_router_loop,
    };

    #[tokio::test]
    async fn ui_command_router_survives_broadcast_lag_without_silent_exit() {
        let gw: Arc<dyn marlin_agent_app_server::mygo_host::config::LiveRouteGateway> = Arc::new(DeterministicLocalGateway {
            chunk_delay: Duration::from_millis(0),
        });
        let mut routes = BTreeMap::new();
        routes.insert(
            "default".to_owned(),
            marlin_agent_app_server::mygo_host::config::RouteEntry {
                label: "alpha-model".to_owned(),
                endpoint: ModelEndpoint::new("openai", "alpha"),
                standard_gateway: Arc::clone(&gw),
                thinking_gateway: None,
            },
        );
        routes.insert(
            "fast".to_owned(),
            marlin_agent_app_server::mygo_host::config::RouteEntry {
                label: "beta-fast-model".to_owned(),
                endpoint: ModelEndpoint::new("openai", "beta"),
                standard_gateway: gw,
                thinking_gateway: None,
            },
        );
        let router = Arc::new(
            marlin_agent_app_server::mygo_host::config::HotSwappableModelGateway::new("default".to_owned(), routes, false)
                .expect("router"),
        );
        let host = MyGoBridgeHost::new_default_trust(|_| Ok(()));
        let (tx, rx) = tokio::sync::broadcast::channel::<DispatchedUiCommand>(2);

        // Overflow broadcast capacity=2 before consumer polls -> triggers RecvError::Lagged.
        for i in 0..8usize {
            let _ = tx.send(DispatchedUiCommand {
                command: format!("noop.{i}"),
                session_id: "sess-lag".to_owned(),
                generation: i as u64,
            });
        }
        let _ = tx.send(DispatchedUiCommand {
            command: "model.select fast".to_owned(),
            session_id: "sess-lag".to_owned(),
            generation: 99,
        });
        drop(tx);

        run_ui_command_router_loop(rx, Arc::clone(&router), host).await;
        assert_eq!(
            router.active_alias(),
            "fast",
            "SM5c: UI command router must survive RecvError::Lagged and still process model.select"
        );
        assert_eq!(router.active_label(), "beta-fast-model");
    }

    #[tokio::test]
    async fn host_session_table_evicts_lru_without_permanent_64_slot_lockout() {
        let storage = InMemoryAgentStorage::default();
        let project = ProjectId::new("default").expect("project");
        let agent = AgentId::new("assistant").expect("agent");
        let bindings = Mutex::new(ResidentSessionTable::<InMemoryAgentStorage>::new());

        // Create 128 sequential sessions (2x the 64 resident cache ceiling).
        // SM3c (BoundedProjectionEviction): creating session 65..128 MUST evict
        // idle resident slots instead of permanently locking out with ResourceExhausted.
        for idx in 0..128usize {
            let sid = marlin_agent_storage::SessionId::new(format!("sess-{idx:04}"))
                .expect("session id");
            lookup_host_session(
                &bindings,
                storage.clone(),
                project.clone(),
                agent.clone(),
                sid,
                SessionOpen::Create,
            )
            .await
            .unwrap_or_else(|err| {
                panic!("SM3c permanent slot lockout at session {idx}: {err}")
            });
            let resident = bindings.lock().await.len();
            assert!(
                resident <= MAX_RESIDENT_SESSIONS,
                "resident session cache exceeded 64 ceiling: {resident}"
            );
        }

        // Duplicate Create of an evicted session still fails closed via durable storage.
        let oldest = marlin_agent_storage::SessionId::new("sess-0000").expect("sid");
        let dup_err = lookup_host_session(
            &bindings,
            storage.clone(),
            project.clone(),
            agent.clone(),
            oldest.clone(),
            SessionOpen::Create,
        )
        .await
        .err()
        .expect("duplicate Create on evicted durable session must be rejected");
        assert_eq!(dup_err.code(), tonic::Code::AlreadyExists);

        // Re-opening evicted session 0 via Existing MUST rehydrate cleanly from durable storage.
        lookup_host_session(
            &bindings,
            storage.clone(),
            project.clone(),
            agent.clone(),
            oldest,
            SessionOpen::Existing,
        )
        .await
        .expect("evicted session 0 must rehydrate from durable storage without lockout");
        assert!(bindings.lock().await.len() <= MAX_RESIDENT_SESSIONS);
    }
}

mod config_tests {
    use std::fs;

    use agent_semantic_search::{WorkspaceIndexConfig, WorkspaceSearchIndex};

    use super::{
        Arc, BTreeMap, GraphVisibleSurface, HostModelConfig, HotSwappableModelGateway,
        ModelEndpoint, ModelGateway, ModelGatewayCompletionResponse, ModelGatewayEventStream,
        ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, ModelStreamingGateway,
        ParsedHelmQuery, QueryProjectGraphRequest, RouteEntry, WorkspaceHelmSearchBridge,
    };

    #[test]
    fn parses_multi_axis_helm_query_and_strips_local_stage() {
        let parsed = ParsedHelmQuery::parse(
            "@lang:rust @prefix:crates/ @ext:.rs @sym:MyGoBridgeHost helm #local !test",
        );
        assert_eq!(parsed.languages, vec!["rust"]);
        assert_eq!(parsed.path_prefix.as_deref(), Some("crates/"));
        assert_eq!(parsed.extension.as_deref(), Some("rs"));
        assert_eq!(parsed.symbol.as_deref(), Some("MyGoBridgeHost"));
        assert_eq!(parsed.pattern, "helm");
    }

    #[test]
    fn parses_tui_example_toml_with_optional_host_section() {
        let raw = r#"
provider = "deepseek"
model = "deepseek-flash"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"
timeout_seconds = 120

[anthropic]
thinking = { type = "adaptive" }
effort = "low"
max_tokens = 8192

[models.deepseek-v4-pro]
provider = "deepseek"
model = "deepseek-v4-pro"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"

[host]
addr = "127.0.0.1:50051"

[surface]
glass = true
vibrancy = "under-window"
glass_opacity = 0.24
chrome_opacity = 0.14
viewport_opacity = 0.20
"#;
        let cfg: HostModelConfig = toml::from_str(raw).expect("valid TOML config");
        assert_eq!(cfg.provider, "deepseek");
        assert_eq!(cfg.model.as_deref(), Some("deepseek-flash"));
        assert!(cfg.models.contains_key("deepseek-v4-pro"));
    }

    #[test]
    fn workspace_helm_bridge_returns_real_indexed_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("lib.rs"),
            "pub fn marlin_helm_probe() -> u32 { 42 }\n",
        )
        .expect("write rust fixture");
        let snap = dir.path().join("cache.asp-index.bin");
        let idx = WorkspaceSearchIndex::open_or_build_cached(
            dir.path(),
            &snap,
            WorkspaceIndexConfig::default(),
        );
        let bridge = WorkspaceHelmSearchBridge::from_index(idx);
        let req = QueryProjectGraphRequest {
            project_id: "default".to_owned(),
            session_id: None,
            surfaces: vec![GraphVisibleSurface::Workspace as i32],
            secret_visibility: 0,
            query: "@lang:rust marlin_helm_probe".to_owned(),
            page: None,
        };
        let records = bridge.query(&req).expect("index ready");
        assert!(!records.is_empty(), "expected live search hit");
        assert_eq!(records[0].source_path.as_deref(), Some("lib.rs"));
        assert!(records[0].facets.contains_key("generation_digest"));
    }

    #[tokio::test]
    async fn hot_swappable_gateway_switches_aliases_and_thinking_mode_live() {
        use marlin_agent_protocol::{
            ModelGatewayCompletionChoice, assistant_gateway_message, user_gateway_message,
        };
        use marlin_agent_stream::{ModelStreamChunk, ModelStreamEvent};

        #[derive(Clone)]
        struct TaggedGateway(&'static str);

        impl ModelGateway for TaggedGateway {
            fn complete(
                &self,
                _request: ModelGatewayRequest,
            ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
                let tag = self.0;
                Box::pin(async move {
                    Ok(ModelGatewayCompletionResponse::new(
                        tag,
                        tag,
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message(tag.to_owned()),
                            None,
                        )],
                    ))
                })
            }
        }

        impl ModelStreamingGateway for TaggedGateway {
            fn stream(
                &self,
                _request: ModelGatewayRequest,
            ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
                let tag = self.0;
                Box::pin(async move {
                    let s = async_stream::try_stream! {
                        yield ModelStreamEvent::Chunk(ModelStreamChunk::new(1, tag.to_owned()));
                    };
                    Ok(Box::pin(s) as ModelGatewayEventStream)
                })
            }
        }

        let mut routes = BTreeMap::new();
        routes.insert(
            "default".to_owned(),
            RouteEntry {
                label: "deepseek/deepseek-flash".to_owned(),
                endpoint: ModelEndpoint::new("deepseek", "deepseek-flash"),
                standard_gateway: Arc::new(TaggedGateway("flash-std")),
                thinking_gateway: Some(Arc::new(TaggedGateway("flash-think"))),
            },
        );
        routes.insert(
            "deepseek-v4-pro".to_owned(),
            RouteEntry {
                label: "deepseek-v4-pro · deepseek-reasoner (deepseek)".to_owned(),
                endpoint: ModelEndpoint::new("deepseek", "deepseek-reasoner"),
                standard_gateway: Arc::new(TaggedGateway("pro-std")),
                thinking_gateway: Some(Arc::new(TaggedGateway("pro-think"))),
            },
        );

        let router = HotSwappableModelGateway::new("default".to_owned(), routes, false)
            .expect("valid router");
        assert_eq!(router.active_alias(), "default");
        assert_eq!(router.available_aliases().len(), 2);

        let req = ModelGatewayRequest::new(
            ModelEndpoint::new("deepseek", "deepseek-flash"),
            vec![user_gateway_message("ping")],
        );
        let r1 = router.complete(req.clone()).await.expect("std complete");
        assert_eq!(r1.choices[0].message.content, "flash-std");

        // Enable Deep Think live -> routes to thinking_gateway immediately!
        assert!(router.set_thinking(true));
        let r2 = router.complete(req.clone()).await.expect("think complete");
        assert_eq!(r2.choices[0].message.content, "flash-think");

        // Hot-switch alias live -> routes to deepseek-v4-pro thinking gateway!
        let lbl = router
            .select_alias("deepseek-v4-pro")
            .expect("known alias");
        assert_eq!(lbl, "deepseek-v4-pro · deepseek-reasoner (deepseek)");
        assert_eq!(router.active_label(), lbl);
        let r3 = router.complete(req.clone()).await.expect("pro think complete");
        assert_eq!(r3.choices[0].message.content, "pro-think");

        // Unknown alias fails closed without disturbing active route.
        assert!(router.select_alias("nonexistent-model").is_none());
        assert_eq!(router.active_alias(), "deepseek-v4-pro");
    }
}
