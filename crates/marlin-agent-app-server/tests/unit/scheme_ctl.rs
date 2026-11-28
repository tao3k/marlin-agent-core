// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Unit tests for `marlin_agent_app_server::scheme_ctl::args`.

use marlin_agent_app_server::{
    FleetTaskAction, GovernanceFactAction,
    scheme_ctl::args::{
        parse_cli_args, parse_fleet_task_action, parse_gov_fact_action, parse_probe_cedar_args,
        parse_records_flags, parse_wait_ready_flags, parse_watch_flags,
    },
};


    #[test]
    fn parse_cli_args_supports_json_endpoint_and_token_flags() {
        let args = [
            "--json",
            "-e",
            "http://127.0.0.1:9001",
            "--token",
            "sec-1",
            "status",
        ]
        .into_iter()
        .map(str::to_owned);
        let parsed = parse_cli_args(args, None, None).expect("parse cli args");
        assert!(parsed.json);
        assert!(!parsed.help);
        assert_eq!(parsed.endpoint.as_deref(), Some("http://127.0.0.1:9001"));
        assert_eq!(parsed.token.as_deref(), Some("sec-1"));
        assert_eq!(parsed.rest, vec!["status".to_owned()]);
    }

    #[test]
    fn parse_cli_args_rejects_missing_flag_values() {
        assert!(parse_cli_args(["--endpoint".to_owned()].into_iter(), None, None).is_err());
        assert!(parse_cli_args(["--token".to_owned()].into_iter(), None, None).is_err());
    }

    #[test]
    fn parse_watch_flags_validates_bounds_and_unknown_options() {
        let ok = parse_watch_flags(&[
            "--count".to_owned(),
            "2".to_owned(),
            "--timeout-ms".to_owned(),
            "3000".to_owned(),
            "--session".to_owned(),
            "s-1".to_owned(),
        ])
        .expect("valid watch flags");
        assert_eq!(ok.count, Some(2));
        assert_eq!(ok.timeout_ms, Some(3000));
        assert_eq!(ok.session.as_deref(), Some("s-1"));

        assert!(parse_watch_flags(&["--count".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_watch_flags(&["--bogus".to_owned()]).is_err());
    }

    #[test]
    fn parse_wait_ready_and_records_flags_validate_inputs() {
        let wr = parse_wait_ready_flags(&[
            "--timeout-ms".to_owned(),
            "2500".to_owned(),
            "--interval-ms".to_owned(),
            "50".to_owned(),
        ])
        .expect("valid wait-ready flags");
        assert_eq!(wr.timeout_ms, 2500);
        assert_eq!(wr.interval_ms, 50);
        assert!(parse_wait_ready_flags(&["--timeout-ms".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_wait_ready_flags(&["--interval-ms".to_owned(), "0".to_owned()]).is_err());
        assert!(parse_wait_ready_flags(&["--nope".to_owned()]).is_err());

        let rec = parse_records_flags(&[
            "--project".to_owned(),
            "alpha".to_owned(),
            "--kind".to_owned(),
            "scheme.fleet.".to_owned(),
        ])
        .expect("valid records flags");
        assert_eq!(rec.project.as_deref(), Some("alpha"));
        assert_eq!(rec.kind_prefix.as_deref(), Some("scheme.fleet."));
        assert!(parse_records_flags(&["--kind".to_owned()]).is_err());
        assert!(parse_records_flags(&["--bad".to_owned()]).is_err());
    }

    #[test]
    fn parse_gov_fact_action_validates_all_variants_and_rejects_rearm() {
        let s = |vals: &[&str]| vals.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            parse_gov_fact_action(&s(&["eval-clean"])).unwrap(),
            GovernanceFactAction::EvalClean
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["delegate", "ops-root", "agent-lead"])).unwrap(),
            GovernanceFactAction::Delegate {
                parent: "ops-root".to_owned(),
                child: "agent-lead".to_owned(),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["taint-edge", "in", "pipe", "source"])).unwrap(),
            GovernanceFactAction::TaintEdge {
                src: "in".to_owned(),
                dst: "pipe".to_owned(),
                role: Some("source".to_owned()),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["sanitize", "pipe"])).unwrap(),
            GovernanceFactAction::Sanitize {
                node: "pipe".to_owned(),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&["check-cedar", "worker", "run_tool", "workspace"])).unwrap(),
            GovernanceFactAction::CheckCedar {
                actor: "worker".to_owned(),
                action: "run_tool".to_owned(),
                resource: "workspace".to_owned(),
                step_up_fresh: false,
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&[
                "check-cedar",
                "alice",
                "rearm_breaker",
                "workspace",
                "true",
            ]))
            .unwrap(),
            GovernanceFactAction::CheckCedar {
                actor: "alice".to_owned(),
                action: "rearm_breaker".to_owned(),
                resource: "workspace".to_owned(),
                step_up_fresh: true,
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&[
                "register-derivation",
                "D-TOKEN",
                "pol.cert",
                "token-cert-v1",
                "none",
            ]))
            .unwrap(),
            GovernanceFactAction::RegisterDerivation {
                derivation_id: "D-TOKEN".to_owned(),
                policy_id: "pol.cert".to_owned(),
                subject_premises: vec!["token-cert-v1".to_owned()],
                parent_derivations: Vec::new(),
            }
        );
        assert_eq!(
            parse_gov_fact_action(&s(&[
                "register-derivation",
                "D-PLAN",
                "pol.plan",
                "workspace-ok,scope-ok",
                "D-TOKEN",
            ]))
            .unwrap(),
            GovernanceFactAction::RegisterDerivation {
                derivation_id: "D-PLAN".to_owned(),
                policy_id: "pol.plan".to_owned(),
                subject_premises: vec!["workspace-ok".to_owned(), "scope-ok".to_owned()],
                parent_derivations: vec!["D-TOKEN".to_owned()],
            }
        );
        assert!(parse_gov_fact_action(&s(&["rearm", "alice"])).is_err());
        assert!(parse_gov_fact_action(&s(&["taint-edge", "a", "b", "bogus"])).is_err());
        assert!(parse_gov_fact_action(&s(&["delegate", "only-one"])).is_err());
        assert!(parse_gov_fact_action(&s(&["check-cedar", "alice", "run_tool"])).is_err());
        assert!(
            parse_gov_fact_action(&s(&["check-cedar", "alice", "run_tool", "ws", "maybe"]))
                .is_err()
        );
        assert!(
            parse_gov_fact_action(&s(&["register-derivation", "D-BAD", "pol.x", "none"])).is_err()
        );
        assert!(parse_gov_fact_action(&s(&["register-derivation", "D-BAD", "pol.x"])).is_err());
    }

    #[test]
    fn parse_fleet_task_action_validates_all_variants_and_rejects_bypass() {
        let s = |vals: &[&str]| vals.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            parse_fleet_task_action(&s(&["drill", "0"])).unwrap(),
            FleetTaskAction::Drill { level: 0 }
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["drill", "3"])).unwrap(),
            FleetTaskAction::Drill { level: 3 }
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["propose-branch", "B1:Verify", "lineage"])).unwrap(),
            FleetTaskAction::ProposeBranch {
                label: Some("B1:Verify lineage".to_owned()),
            }
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["detour-blocker", "blk-1"])).unwrap(),
            FleetTaskAction::DetourBlocker {
                note: Some("blk-1".to_owned()),
            }
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["detour-exit", "return"])).unwrap(),
            FleetTaskAction::DetourExitReturn
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["detour-exit", "sibling"])).unwrap(),
            FleetTaskAction::DetourExitSibling
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["peek", "actor-2"])).unwrap(),
            FleetTaskAction::Peek {
                actor_id: Some("actor-2".to_owned()),
            }
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["fleet-step"])).unwrap(),
            FleetTaskAction::FleetStep
        );
        assert_eq!(
            parse_fleet_task_action(&s(&["sandbox-verify"])).unwrap(),
            FleetTaskAction::SandboxVerify
        );
        assert!(parse_fleet_task_action(&s(&["drill", "4"])).is_err());
        assert!(parse_fleet_task_action(&s(&["detour-exit", "force"])).is_err());
        assert!(parse_fleet_task_action(&s(&["human-approve", "gate-1"])).is_err());
        assert!(parse_fleet_task_action(&s(&["governance-rearm", "alice"])).is_err());
    }

    #[test]
    fn parse_probe_cedar_args_validates_flags_and_expectations() {
        let s = |vals: &[&str]| vals.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
        let ok = parse_probe_cedar_args(&s(&[
            "alice",
            "rearm_breaker",
            "webhook-out",
            "true",
            "--expect",
            "deny",
            "--timeout-ms",
            "3000",
        ]))
        .expect("valid probe-cedar args");
        assert_eq!(ok.actor, "alice");
        assert_eq!(ok.action, "rearm_breaker");
        assert_eq!(ok.resource, "webhook-out");
        assert!(ok.step_up_fresh);
        assert_eq!(ok.expect.as_deref(), Some("deny"));
        assert_eq!(ok.timeout_ms, 3000);

        let default_step_up =
            parse_probe_cedar_args(&s(&["worker", "run_tool", "workspace"])).expect("default");
        assert!(!default_step_up.step_up_fresh);
        assert_eq!(default_step_up.expect, None);

        assert!(parse_probe_cedar_args(&s(&["alice", "run_tool"])).is_err());
        assert!(parse_probe_cedar_args(&s(&["a", "b", "c", "maybe"])).is_err());
        assert!(parse_probe_cedar_args(&s(&["a", "b", "c", "--expect", "permit"])).is_err());
        assert!(parse_probe_cedar_args(&s(&["a", "b", "c", "--timeout-ms", "0"])).is_err());
    }
