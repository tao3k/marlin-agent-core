// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::fs;

use marlin_agent_harness::live_weaver::{
    CompiledHarnessGovernanceGate, LiveWeaverTomlConfig, ProjectedCedarPolicySpec,
    ProjectedHarnessGovernance,
};

fn read_only_governance() -> ProjectedHarnessGovernance {
    ProjectedHarnessGovernance {
        policy_id: "sm35-readonly-eval".to_owned(),
        write_allowed: false,
        allowed_capabilities: vec!["search".to_owned()],
        denied_capabilities: vec!["write".to_owned(), "shell".to_owned()],
        allowed_tools: Vec::new(),
        denied_tools: vec!["bash".to_owned(), "write_file".to_owned()],
        cedar_policies: vec![
            ProjectedCedarPolicySpec {
                policy_id: "permit-search".to_owned(),
                source: r#"permit(principal, action == Marlin::Action::"tool.search", resource);"#
                    .to_owned(),
            },
            ProjectedCedarPolicySpec {
                policy_id: "forbid-write".to_owned(),
                source: r#"forbid(principal, action, resource) when { context.is_write };"#.to_owned(),
            },
        ],
        denial_message: "Write or shell tools are forbidden in read-only eval.".to_owned(),
    }
}

#[test]
fn sm35_v3_cedar_governance_gate_refuses_capability_spoofing_substring_and_case_bypass() {
    let gate = CompiledHarnessGovernanceGate::compile(read_only_governance())
        .expect("compile read-only Cedar governance");

    // 1. Substring "search" smuggling in destructive tool names MUST fail closed as write/shell.
    let search_exec = gate.evaluate_tool_call("s1", "c1", "search_exec", None);
    assert!(
        !search_exec.permitted && search_exec.is_write,
        "SM35 V3: `search_exec` MUST be denied as write-capable, got {search_exec:?}"
    );

    // 2. Caller-supplied `declared_capability = Some("search")` MUST NOT downgrade destructive/exec/rm tools.
    let spoofed_exec = gate.evaluate_tool_call("s1", "c2", "exec_command", Some("search"));
    assert!(
        !spoofed_exec.permitted && spoofed_exec.is_write,
        "SM35 V3: `exec_command` with spoofed `Some(\"search\")` MUST be denied, got {spoofed_exec:?}"
    );

    let spoofed_rm = gate.evaluate_tool_call("s1", "c3", "rm_file", Some("search"));
    assert!(
        !spoofed_rm.permitted && spoofed_rm.is_write,
        "SM35 V3: `rm_file` with spoofed `Some(\"search\")` MUST be denied, got {spoofed_rm:?}"
    );

    // 3. Case-sensitivity bypass on denied_tools (`BASH` vs `bash`) MUST fail closed.
    let upper_bash = gate.evaluate_tool_call("s1", "c4", "BASH", Some("search"));
    assert!(
        !upper_bash.permitted && upper_bash.is_write,
        "SM35 V3: uppercase `BASH` with spoofed capability MUST be denied, got {upper_bash:?}"
    );

    // 4. Empty Cedar policies or contradictory write_allowed=false + allowed_capabilities=["write"] MUST fail compile().
    let mut empty_cedar = read_only_governance();
    empty_cedar.cedar_policies.clear();
    assert!(
        CompiledHarnessGovernanceGate::compile(empty_cedar).is_err(),
        "SM35 V3: empty Cedar policy list MUST fail compile()"
    );

    let mut contradictory = read_only_governance();
    contradictory.allowed_capabilities = vec!["search".to_owned(), "write".to_owned()];
    assert!(
        CompiledHarnessGovernanceGate::compile(contradictory).is_err(),
        "SM35 V3: write_allowed=false with allowed_capabilities containing `write` MUST fail compile()"
    );

    // 5. Positive control: legitimate read-only search tools remain permitted.
    for safe in ["workspace_search", "structural_query", "constraint_solve"] {
        let ok = gate.evaluate_tool_call("s1", "c-ok", safe, Some("search"));
        assert!(ok.permitted && !ok.is_write, "safe tool `{safe}` must stay permitted: {ok:?}");
    }
}

#[test]
fn sm35_v5_weaver_toml_config_refuses_oversize_symlink_bad_timeout_and_ambient_cred_env() {
    let dir = tempfile::tempdir().expect("create tempdir");

    // 1. Oversize TOML config (> 256 KiB) MUST fail closed.
    let big_path = dir.path().join("huge.toml");
    let big_body = format!(
        "provider = \"deepseek\"\nprotocol = \"anthropic\"\ncredential_env = \"ANTHROPIC_API_KEY\"\n# {}\n",
        "A".repeat(300 * 1024)
    );
    fs::write(&big_path, big_body).expect("write huge toml");
    assert!(
        LiveWeaverTomlConfig::load(&big_path).is_err(),
        "SM35 V5: >256 KiB TOML config MUST fail closed"
    );

    // 2. Symlinked TOML config MUST fail closed.
    #[cfg(unix)]
    {
        let real_path = dir.path().join("real.toml");
        fs::write(
            &real_path,
            "provider = \"deepseek\"\nprotocol = \"anthropic\"\ncredential_env = \"ANTHROPIC_API_KEY\"\n",
        )
        .expect("write real toml");
        let link_path = dir.path().join("link.toml");
        symlink(&real_path, &link_path).expect("plant symlink");
        assert!(
            LiveWeaverTomlConfig::load(&link_path).is_err(),
            "SM35 V5: symlinked TOML config MUST fail closed"
        );
    }

    // 3. Zero or extreme timeout_seconds MUST fail closed.
    let zero_timeout_toml = r#"
provider = "deepseek"
model = "deepseek-flash"
endpoint = "https://api.deepseek.com/anthropic"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"
timeout_seconds = 0
"#;
    let zero_cfg: LiveWeaverTomlConfig = toml::from_str(zero_timeout_toml).expect("parse toml");
    assert!(
        zero_cfg
            .resolve_route_with("default", 65_536, |_| Some("secret".to_owned()))
            .is_err(),
        "SM35 V5: timeout_seconds = 0 MUST fail closed"
    );

    // 4. Dangerous ambient credential_env (`PATH`, `HOME`, `LD_PRELOAD`, CRLF) MUST fail closed.
    for bad_env in ["PATH", "HOME", "LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "KEY\r\nBAD", ""] {
        let bad_toml = format!(
            "provider = \"deepseek\"\nmodel = \"deepseek-flash\"\nendpoint = \"https://api.deepseek.com/anthropic\"\nprotocol = \"anthropic\"\ncredential_env = {bad_env:?}\ntimeout_seconds = 30\n"
        );
        let cfg: LiveWeaverTomlConfig = toml::from_str(&bad_toml).expect("parse toml");
        assert!(
            cfg.resolve_route_with("default", 65_536, |_| Some("/usr/bin".to_owned()))
                .is_err(),
            "SM35 V5: ambient/malformed credential_env `{bad_env}` MUST fail closed"
        );
    }

    // 5. Unbounded max_stream_bytes MUST fail closed.
    let valid_toml = r#"
provider = "deepseek"
model = "deepseek-flash"
endpoint = "https://api.deepseek.com/anthropic"
protocol = "anthropic"
credential_env = "ANTHROPIC_API_KEY"
timeout_seconds = 30
"#;
    let valid_cfg: LiveWeaverTomlConfig = toml::from_str(valid_toml).expect("parse valid toml");
    assert!(
        valid_cfg
            .resolve_route_with("default", usize::MAX, |_| Some("test-key".to_owned()))
            .is_err(),
        "SM35 V5: max_stream_bytes = usize::MAX MUST fail closed"
    );
}
