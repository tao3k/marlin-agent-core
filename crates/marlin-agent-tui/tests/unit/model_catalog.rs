// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::{
    config::Config,
    journal_gateway::JournalGateway,
    model_catalog::{DEFAULT_MODEL_ALIAS, ModelCatalog, ModelRoute},
};
use marlin_agent_protocol::{
    ModelAlias, ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, assistant_gateway_message,
    user_gateway_message,
};
use marlin_agent_sessions::ConversationJournal;
use marlin_agent_storage::{AgentId, InMemoryAgentStorage, ProjectId, SessionId};
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamEvent, ModelStreamingGateway};
use tokio_stream::StreamExt;

struct Counter(Arc<AtomicUsize>);
impl ModelStreamingGateway for Counter {
    fn stream(
        &self,
        _: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(
                Box::pin(tokio_stream::iter(vec![Ok(ModelStreamEvent::Completed {
                    response: ModelGatewayCompletionResponse::new(
                        "fixture",
                        "model",
                        vec![ModelGatewayCompletionChoice::new(
                            0,
                            assistant_gateway_message("fixture response"),
                            None,
                        )],
                    ),
                })])) as ModelGatewayEventStream,
            )
        })
    }
}

fn route(name: &str, provider: &str, model: &str, calls: Option<Arc<AtomicUsize>>) -> ModelRoute {
    let alias = ModelAlias::new(name);
    let mut endpoint = ModelEndpoint::new(provider, model);
    if name != DEFAULT_MODEL_ALIAS {
        endpoint = endpoint.with_alias(alias.clone());
    }
    ModelRoute {
        alias,
        endpoint,
        gateway: calls.map(|calls| Arc::new(Counter(calls)) as Arc<dyn ModelStreamingGateway>),
    }
}

#[tokio::test]
async fn catalog_routes_only_exact_selected_identity_and_denies_unavailable_models() {
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));
    let catalog = ModelCatalog::new(vec![
        route("default", "provider", "one", Some(first.clone())),
        route("alternate", "provider", "two", Some(second.clone())),
        route("disabled", "provider", "three", None),
    ])
    .unwrap();
    for name in ["default", "alternate"] {
        let endpoint = catalog.select(&ModelAlias::new(name)).unwrap();
        assert!(
            catalog
                .stream(ModelGatewayRequest::new(
                    endpoint,
                    vec![user_gateway_message("hi")]
                ))
                .await
                .is_ok()
        );
    }
    assert!(catalog.select(&ModelAlias::new("disabled")).is_err());
    assert!(catalog.select(&ModelAlias::new("missing")).is_err());
    let forged = ModelEndpoint::new("provider", "one").with_alias("alternate");
    assert!(
        catalog
            .stream(ModelGatewayRequest::new(
                forged,
                vec![user_gateway_message("hi")]
            ))
            .await
            .is_err()
    );
    assert_eq!(first.load(Ordering::SeqCst), 1);
    assert_eq!(second.load(Ordering::SeqCst), 1);
}

#[test]
fn catalog_rejects_invalid_aliases_duplicates_and_unavailable_default() {
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(ModelCatalog::new(vec![route("default", "provider", "one", None)]).is_err());
    assert!(
        ModelCatalog::new(vec![
            route("default", "provider", "one", Some(calls.clone())),
            route("bad alias", "provider", "two", None)
        ])
        .is_err()
    );
    assert!(
        ModelCatalog::new(vec![
            route("default", "provider", "one", Some(calls.clone())),
            route("default", "provider", "two", Some(calls))
        ])
        .is_err()
    );
}

#[test]
fn anthropic_missing_endpoint_uses_weaver_environment_fallback() {
    let config: Config = toml::from_str(
        r#"
provider = "deepseek"
model = "deepseek-v4-pro[1m]"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"
"#,
    )
    .unwrap();
    let env = |key: &str| match key {
        "ANTHROPIC_BASE_URL" => Some("http://127.0.0.1:1234/anthropic".to_owned()),
        "ANTHROPIC_API_KEY" => Some("fixture".to_owned()),
        "ANTHROPIC_MODEL" => Some("must-not-override-config".to_owned()),
        _ => None,
    };
    let (endpoint, _) = config.catalog_with(env).unwrap();
    assert_eq!(endpoint.model.as_str(), "deepseek-v4-pro[1m]");
    assert!(
        config
            .catalog_with(|key| (key == "ANTHROPIC_API_KEY").then(|| "fixture".to_owned()))
            .is_err()
    );
}

#[test]
fn anthropic_missing_model_uses_environment_model() {
    let config: Config = toml::from_str(
        r#"
provider = "deepseek"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"
"#,
    )
    .unwrap();
    let (endpoint, _) = config
        .catalog_with(|key| match key {
            "ANTHROPIC_BASE_URL" => Some("http://127.0.0.1:1234".into()),
            "ANTHROPIC_API_KEY" => Some("fixture".into()),
            "ANTHROPIC_MODEL" => Some("deepseek-env-model".into()),
            _ => None,
        })
        .unwrap();
    assert_eq!(endpoint.model.as_str(), "deepseek-env-model");
}

#[test]
fn anthropic_settings_are_validated_by_the_provider_library() {
    let base = r#"
provider = "deepseek"
model = "fixture"
endpoint = "http://127.0.0.1:1234/v1/messages"
protocol = "anthropic"
credential_env = "KEY"
[anthropic]
"#;
    let env = |key: &str| (key == "KEY").then(|| "fixture".to_owned());
    for settings in [
        "thinking = { type = 'adaptive' }\neffort = 'low'\nmax_tokens = 8192",
        "thinking = { type = 'enabled', budget_tokens = 2048 }\nmax_tokens = 4096",
        "thinking = { type = 'disabled' }",
    ] {
        let config: Config = toml::from_str(&format!("{base}{settings}")).unwrap();
        assert!(config.catalog_with(env).is_ok());
    }
    for settings in [
        "thinking = { type = 'enabled', budget_tokens = 512 }\nmax_tokens = 4096",
        "thinking = { type = 'enabled', budget_tokens = 2048 }\nmax_tokens = 2048",
        "max_tokens = 0",
    ] {
        let config: Config = toml::from_str(&format!("{base}{settings}")).unwrap();
        assert!(config.catalog_with(env).is_err());
    }
    let invalid_protocol = base.replace("protocol = \"anthropic\"", "protocol = \"open-ai\"");
    let config: Config = toml::from_str(&format!("{invalid_protocol}effort = 'low'")).unwrap();
    assert!(config.catalog_with(env).is_err());
    assert!(
        toml::from_str::<Config>(&format!(
            "{base}thinking = {{ type = 'adaptive', typo = 1 }}"
        ))
        .is_err()
    );
}

#[test]
fn explicit_anthropic_endpoint_takes_precedence_over_environment() {
    let config: Config = toml::from_str(
        r#"
provider = "deepseek"
model = "deepseek-fixture"
endpoint = "http://127.0.0.1:1234/v1/messages"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"
"#,
    )
    .unwrap();
    assert!(
        config
            .catalog_with(|key| match key {
                "ANTHROPIC_BASE_URL" => Some("not a URL".into()),
                "ANTHROPIC_API_KEY" => Some("fixture".into()),
                _ => None,
            })
            .is_ok()
    );
}

#[test]
fn configured_profiles_distinguish_missing_optional_credentials_without_leaking_them() {
    let config: Config = toml::from_str(
        r#"
provider = "openai"
model = "gpt-first"
endpoint = "http://127.0.0.1:1111/v1/responses"
protocol = "open-ai-responses"
credential_env = "PRIMARY"
[models.alt]
provider = "openai"
model = "gpt-second"
endpoint = "http://127.0.0.1:2222/v1/responses"
protocol = "open-ai-responses"
credential_env = "OPTIONAL"
"#,
    )
    .unwrap();
    let (default, catalog) = config
        .catalog_with(|key| (key == "PRIMARY").then(|| "secret-fixture".to_string()))
        .unwrap();
    assert!(default.alias.is_none());
    assert!(catalog.select(&ModelAlias::new("alt")).is_err());
    let listed = catalog
        .entries()
        .map(|(alias, _, available)| (alias.as_str().to_string(), available))
        .collect::<Vec<_>>();
    assert_eq!(
        listed,
        [("alt".to_string(), false), ("default".to_string(), true)]
    );
    assert!(config.catalog_with(|_| None).is_err());
    let (_, ready) = config
        .catalog_with(|_| Some("secret-fixture".into()))
        .unwrap();
    assert_eq!(
        ready
            .select(&ModelAlias::new("alt"))
            .unwrap()
            .alias
            .unwrap()
            .as_str(),
        "alt"
    );
}

#[tokio::test]
async fn journal_records_the_selected_endpoint_for_each_dispatched_turn() {
    let calls = Arc::new(AtomicUsize::new(0));
    let catalog = ModelCatalog::new(vec![
        route("default", "provider", "one", Some(calls.clone())),
        route("alternate", "provider", "two", Some(calls)),
    ])
    .unwrap();
    let storage = InMemoryAgentStorage::new();
    let project = ProjectId::new("project:models").unwrap();
    let session = SessionId::new("session:models").unwrap();
    let agent = AgentId::new("agent:models").unwrap();
    let journal = ConversationJournal::open(
        storage.clone(),
        project.clone(),
        session.clone(),
        agent.clone(),
        |_| {},
    )
    .await
    .unwrap();
    let gateway = JournalGateway::new(catalog.clone(), journal);
    for alias in ["default", "alternate"] {
        let endpoint = catalog.select(&ModelAlias::new(alias)).unwrap();
        let mut stream = gateway
            .stream(ModelGatewayRequest::new(
                endpoint,
                vec![user_gateway_message("hello")],
            ))
            .await
            .unwrap();
        assert!(matches!(
            stream.next().await.unwrap(),
            Ok(ModelStreamEvent::Completed { .. })
        ));
    }
    let mut turns = Vec::new();
    ConversationJournal::open(storage, project, session, agent, |turn| {
        turns.push(turn.clone())
    })
    .await
    .unwrap();
    assert_eq!(
        turns[0].start.endpoint,
        catalog.select(&ModelAlias::new("default")).unwrap()
    );
    assert_eq!(
        turns[1].start.endpoint,
        catalog.select(&ModelAlias::new("alternate")).unwrap()
    );
}

#[tokio::test]
async fn rejected_catalog_route_is_not_dispatched_and_does_not_block_valid_route() {
    let calls = Arc::new(AtomicUsize::new(0));
    let catalog = ModelCatalog::new(vec![route(
        "default",
        "provider",
        "one",
        Some(calls.clone()),
    )])
    .unwrap();
    let storage = InMemoryAgentStorage::new();
    let journal = ConversationJournal::open(
        storage,
        ProjectId::new("project:route-denial").unwrap(),
        SessionId::new("session:route-denial").unwrap(),
        AgentId::new("agent:route-denial").unwrap(),
        |_| {},
    )
    .await
    .unwrap();
    let gateway = JournalGateway::new(catalog.clone(), journal);
    let forged = ModelEndpoint::new("provider", "other");
    assert!(matches!(
        gateway
            .stream(ModelGatewayRequest::new(
                forged,
                vec![user_gateway_message("hello")]
            ))
            .await,
        Err(marlin_agent_protocol::ModelGatewayError::RequestNotDispatched(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let endpoint = catalog.select(&ModelAlias::new("default")).unwrap();
    let mut stream = gateway
        .stream(ModelGatewayRequest::new(
            endpoint,
            vec![user_gateway_message("hello")],
        ))
        .await
        .unwrap();
    assert!(matches!(
        stream.next().await.unwrap(),
        Ok(ModelStreamEvent::Completed { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
