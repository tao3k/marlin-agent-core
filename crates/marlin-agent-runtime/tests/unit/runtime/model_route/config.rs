use std::io::Write;

use marlin_agent_protocol::{
    ModelCommandMatcher, ModelContextForkMode, ModelEndpoint, ModelEndpointContractError,
    ModelRouteRequest, ModelRouteRule, ModelSessionLifecycle, ModelSessionPersistenceKey,
};
use marlin_agent_runtime::{ModelRouteConfig, ModelRouteConfigError};
use tempfile::{NamedTempFile, tempdir};

const ROUTE_CONFIG_TOML: &str = r#"
[[rules]]
rule_id = "cargo-test-cheap"
priority = 10

[rules.matcher]
executable_globs = ["cargo"]
argv_globs = ["cargo test*"]
workspace_globs = ["*/marlin-agent-core"]
sub_agent_role_globs = ["tester"]
command_kind_globs = ["test"]

[rules.endpoint]
provider = "openai"
model = "gpt-5-mini"
alias = "cheap-test"

[rules.session]
context = "Minimal"

[rules.session.lifecycle.Persistent]
key = "workspace:tester"
"#;

#[test]
fn model_route_config_loads_toml_and_compiles_resolver() {
    let config = ModelRouteConfig::from_toml_str(ROUTE_CONFIG_TOML).expect("config parses");

    assert_eq!(config.rules().len(), 1);

    let resolver = config.compile_resolver().expect("resolver compiles");
    let request = ModelRouteRequest::command(["cargo", "test", "-p", "marlin-agent-runtime"])
        .with_workspace("/Users/guangtao/ghq/github.com/tao3k/marlin-agent-core")
        .with_sub_agent_role("tester")
        .with_command_kind("test");
    let decision = resolver.resolve(&request).expect("route resolves");

    assert_eq!(decision.endpoint.provider.as_str(), "openai");
    assert_eq!(decision.endpoint.model.as_str(), "gpt-5-mini");
    assert_eq!(
        decision.receipt.litellm_model_id.as_str(),
        "openai/gpt-5-mini"
    );
    assert_eq!(decision.receipt.context_fork, ModelContextForkMode::Minimal);
    assert_eq!(
        decision.receipt.session_lifecycle,
        ModelSessionLifecycle::Persistent {
            key: ModelSessionPersistenceKey::from("workspace:tester")
        }
    );
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"executable:cargo".to_owned())
    );
    assert!(
        decision
            .receipt
            .matched_globs
            .contains(&"command_kind:test".to_owned())
    );
}

#[test]
fn model_route_config_loads_toml_from_path() {
    let mut config_file = NamedTempFile::with_suffix(".toml").expect("creates config fixture");
    config_file
        .write_all(ROUTE_CONFIG_TOML.as_bytes())
        .expect("writes config fixture");

    let config = ModelRouteConfig::from_toml_path(config_file.path()).expect("config loads");

    assert_eq!(config.rules().len(), 1);
    assert_eq!(config.rules()[0].rule_id.as_str(), "cargo-test-cheap");
}

#[test]
fn model_route_config_reports_toml_parse_errors() {
    let error = ModelRouteConfig::from_toml_str("rules = [").expect_err("invalid TOML");

    assert!(matches!(error, ModelRouteConfigError::Toml(_)));
}

#[test]
fn model_route_config_rejects_codex_as_openai_model_name() {
    let error = ModelRouteConfig::from_toml_str(
        r#"
[[rules]]
rule_id = "bad-openai-model"
priority = 0

[rules.matcher]

[rules.endpoint]
provider = "openai"
model = "codex"
"#,
    )
    .expect_err("Codex product name is not an OpenAI model id");
    let ModelRouteConfigError::EndpointContract { rule_id, source } = error else {
        panic!("expected endpoint contract error");
    };

    assert_eq!(rule_id.as_str(), "bad-openai-model");
    assert_eq!(
        source,
        ModelEndpointContractError::CodexIsNotModelName {
            model: "codex".into()
        }
    );
}

#[test]
fn model_route_config_compile_validates_programmatic_rules() {
    let config = ModelRouteConfig::new(vec![ModelRouteRule::new(
        "bad-anthropic-model",
        0,
        ModelCommandMatcher::new(),
        ModelEndpoint::new("anthropic", "anthropic"),
    )]);

    let error = config
        .compile_resolver()
        .expect_err("provider is not a model id");
    let ModelRouteConfigError::EndpointContract { rule_id, source } = error else {
        panic!("expected endpoint contract error");
    };

    assert_eq!(rule_id.as_str(), "bad-anthropic-model");
    assert_eq!(
        source,
        ModelEndpointContractError::ModelLooksLikeProvider {
            provider: "anthropic".into(),
            model: "anthropic".into()
        }
    );
}

#[test]
fn model_route_config_reports_path_io_errors() {
    let temp_dir = tempdir().expect("creates config fixture directory");
    let path = temp_dir.path().join("missing-marlin-model-route.toml");

    let error = ModelRouteConfig::from_toml_path(&path).expect_err("missing config");
    let ModelRouteConfigError::Io {
        path: error_path, ..
    } = error
    else {
        panic!("expected IO error");
    };

    assert_eq!(error_path, path);
}

#[test]
fn model_route_config_reports_glob_compile_errors() {
    let config = ModelRouteConfig::from_toml_str(
        r#"
[[rules]]
rule_id = "bad-glob"
priority = 0

[rules.matcher]
executable_globs = ["["]

[rules.endpoint]
provider = "openai"
model = "gpt-5-mini"
"#,
    )
    .expect("TOML parses before glob compilation");

    let error = config.into_resolver().expect_err("invalid glob");
    let ModelRouteConfigError::Compile(error) = error else {
        panic!("expected glob compile error");
    };

    assert_eq!(error.dimension(), "executable");
    assert_eq!(error.pattern(), "[");
}
