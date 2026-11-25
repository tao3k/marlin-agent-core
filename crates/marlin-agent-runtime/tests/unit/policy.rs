// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

asp_rust::asp_rust_cargo_test_gate!(mode = deny);

/// Architectural boundary guards ensuring Rust remains a thin, deterministic
/// runtime substrate while ALL high-level and high-order orchestration
/// (FunFlow purity/caching, DAG frontier scheduling, Scoped Detour policy,
/// competing hypothesis evaluation, and Two-Zone prompt assembly) lives in Scheme.
mod thin_rust_runtime_boundary_guard {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        if !dir.exists() {
            return;
        }
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn effect_pump_stays_domain_agnostic_and_forbids_high_order_orchestration() {
        let pump_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/effect_pump.rs");
        let src = fs::read_to_string(&pump_rs).expect("read src/effect_pump.rs");
        // Strip module doc comments so explanatory comments never false-positive;
        // enforce on executable Rust code items and logic.
        let code: String = src
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                !trimmed.starts_with("//")
            })
            .collect::<Vec<_>>()
            .join("\n");

        for forbidden in [
            "Detour",
            "FunFlow",
            "Funflow",
            "Hypothesis",
            "OrgGovernance",
            "DisclosureDepth",
            "TwoZonePrompt",
            "WorkbenchTab",
            "StepPurity",
            "ConvergentTree",
            "ObligationGraph",
            "Ascent",
            "CasMemoized",
        ] {
            assert!(
                !code.contains(forbidden),
                "effect_pump.rs must remain a thin, domain-agnostic atomic I/O pump; forbidden high-order term `{forbidden}` found"
            );
        }
    }

    #[test]
    fn rust_runtime_and_kernel_forbid_reimplementing_scheme_high_order_engines() {
        let runtime_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let kernel_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../marlin-agent-kernel/src");

        let mut files = Vec::new();
        collect_rs_files(&runtime_src, &mut files);
        collect_rs_files(&kernel_src, &mut files);

        let forbidden_high_order_symbols = [
            "CasMemoizedStepHandler",
            "StepPurity",
            "GovernedPromptAssembler",
            "ModelGatewayTaskActorStepHandler",
            "dispatch_frontier_batch",
            "HypothesisBranchEvaluator",
            "CompetingHypothesisMatrix",
            "FunflowMemoizer",
            "DagFrontierScheduler",
            "TwoZonePromptAssembler",
        ];

        let mut violations = Vec::new();
        for file in files {
            let src = fs::read_to_string(&file).unwrap();
            for symbol in forbidden_high_order_symbols {
                if src.contains(symbol) {
                    violations.push(format!(
                        "{}: forbidden high-order Scheme-owned orchestration symbol `{symbol}`",
                        file.display()
                    ));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "High-order orchestration leaked into Rust runtime/kernel! Move this logic into Scheme (`poo-flow`) and emit primitive `AtomicEffect`s instead:\n{}",
            violations.join("\n")
        );
    }
}
