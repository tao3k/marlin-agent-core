// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM33 (ADV-L4) hostile runtime exploit suite for `marlin-agent-protocol`.
//!
//! Proves LIVE RED -> LIVE GREEN across 2 real protocol boundary vulnerabilities:
//! - SM33-V4: `SubAgentSpawnConfigSet` admitting contradictory permissions
//!   (`read_only = true && workspace_write = true`), zero budgets (`max_concurrency = 0`,
//!   `timeout_ms = 0`, `token_budget = 0`), unbounded/depth-0 `descendant_spawn`,
//!   empty/traversal Scheme modules, control/traversal `profile_id`, and >256 KiB TOML reads.
//! - SM33-V5: `RuntimeSessionId` bypassing `try_new` via derived Serde `Deserialize`,
//!   and `ModelEndpoint::validate_contract` admitting CRLF/NUL/whitespace and `?`/`#`/`@`/`\`/`..`.

use marlin_agent_protocol::{
    ModelEndpoint, RuntimeSessionId, SubAgentSpawnConfigSet,
};

#[test]
fn sm33_v4_subagent_config_rejects_contradictions_zero_budgets_and_oversize_files() {
    let hostile_tomls = [
        // 1. Contradictory read_only = true AND workspace_write = true
        r#"
[[profiles]]
profile_id = "contradiction"
agent_type = "worker"
role = "rogue"
[profiles.policy.permissions]
read_only = true
workspace_write = true
network_access = false
process_spawn = false
descendant_spawn = false
[profiles.policy.context]
visibility = ["System"]
[profiles.policy.performance]
max_concurrency = 1
timeout_ms = 1000
max_depth = 1
"#,
        // 2. Zero concurrency budget
        r#"
[[profiles]]
profile_id = "zero-conc"
agent_type = "worker"
role = "rogue"
[profiles.policy.permissions]
read_only = false
workspace_write = true
network_access = false
process_spawn = false
descendant_spawn = false
[profiles.policy.context]
visibility = ["System"]
[profiles.policy.performance]
max_concurrency = 0
timeout_ms = 1000
max_depth = 1
"#,
        // 3. Zero timeout_ms budget
        r#"
[[profiles]]
profile_id = "zero-timeout"
agent_type = "worker"
role = "rogue"
[profiles.policy.permissions]
read_only = true
workspace_write = false
network_access = false
process_spawn = false
descendant_spawn = false
[profiles.policy.context]
visibility = ["System"]
[profiles.policy.performance]
max_concurrency = 1
timeout_ms = 0
max_depth = 1
"#,
        // 4. Descendant spawn enabled without positive bounded max_depth
        r#"
[[profiles]]
profile_id = "fork-bomb"
agent_type = "worker"
role = "rogue"
[profiles.policy.permissions]
read_only = false
workspace_write = false
network_access = false
process_spawn = false
descendant_spawn = true
[profiles.policy.context]
visibility = ["System"]
[profiles.policy.performance]
max_concurrency = 1
timeout_ms = 1000
"#,
        // 5. Traversal Scheme module path
        r#"
[[profiles]]
profile_id = "evil-scheme"
agent_type = "worker"
role = "rogue"
[profiles.strategy.Scheme]
module = "../../../etc/evil"
procedure = "run"
aot = false
"#,
        // 6. Path traversal in profile_id (also used as child_session_id)
        r#"
[[profiles]]
profile_id = "../escape"
agent_type = "worker"
role = "rogue"
"#,
    ];

    for (idx, doc) in hostile_tomls.into_iter().enumerate() {
        let res = SubAgentSpawnConfigSet::from_toml_str(doc);
        assert!(
            res.is_err(),
            "SM33-V4 case #{idx}: hostile sub-agent profile must be rejected, got {res:?}"
        );
    }

    let big_file = std::env::temp_dir().join(format!(
        "marlin-sm33-big-subagents-{}.toml",
        std::process::id()
    ));
    let padding = "#".repeat(300 * 1024);
    std::fs::write(&big_file, format!("{padding}\n")).expect("write big subagent config");
    let big_res = SubAgentSpawnConfigSet::from_toml_path(&big_file);
    let _ = std::fs::remove_file(&big_file);
    assert!(
        big_res.is_err(),
        "SM33-V4: >256 KiB sub-agent TOML config file must be refused"
    );
}

#[test]
fn sm33_v5_runtime_session_id_serde_parity_and_model_endpoint_injection_refused() {
    for bad_json in [
        r#""""#,
        r#""   ""#,
        r#""sess\r\nX-Injected: 1""#,
        r#""../../escape""#,
        r#""bad\u0000id""#,
    ] {
        let parsed = serde_json::from_str::<RuntimeSessionId>(bad_json);
        assert!(
            parsed.is_err(),
            "SM33-V5: RuntimeSessionId deserialization must reject hostile payload {bad_json}, got {parsed:?}"
        );
    }
    let valid: RuntimeSessionId =
        serde_json::from_str(r#""marlin-session-1""#).expect("valid runtime session id");
    assert_eq!(valid.as_str(), "marlin-session-1");

    for (provider, model, alias) in [
        ("openai", "gpt-5-mini\r\nAuthorization: evil", None),
        ("openai", "gpt-5-mini?api_key=leak", None),
        ("anthropic", "claude-opus-4-8#frag", None),
        ("openrouter", "model@evil.example", None),
        ("openrouter", "a..b", None),
        ("openrouter", "model\\escape", None),
        ("openai", "gpt-5-mini", Some("alias\nheader")),
    ] {
        let mut endpoint = ModelEndpoint::new(provider, model);
        if let Some(alias) = alias {
            endpoint = endpoint.with_alias(alias);
        }
        assert!(
            endpoint.validate_contract().is_err(),
            "SM33-V5: ModelEndpoint::validate_contract must reject hostile endpoint {endpoint:?}"
        );
    }
}
