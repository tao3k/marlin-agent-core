use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn core_facade_does_not_depend_on_stream_or_litellm_gateway_crates() {
    for forbidden in ["marlin-agent-stream", "litellm-rs"] {
        assert_dependency_absent("crates/marlin-agent-core", "dependencies", forbidden);
    }
}

#[test]
fn rust_project_harness_policy_stays_on_build_and_test_dependency_planes() {
    for package in [
        "crates/marlin-agent-core",
        "crates/marlin-agent-harness",
        "crates/marlin-agent-test-support",
    ] {
        assert_dependency_absent(
            package,
            "dependencies",
            "marlin-rust-project-harness-policy",
        );
        assert_dependency_present(
            package,
            "dev-dependencies",
            "marlin-rust-project-harness-policy",
        );
        assert_dependency_present(
            package,
            "build-dependencies",
            "marlin-rust-project-harness-policy",
        );
        assert_dependency_absent(package, "dev-dependencies", "rust-lang-project-harness");
        assert_dependency_absent(package, "build-dependencies", "rust-lang-project-harness");
    }
}

#[test]
fn test_support_does_not_depend_on_agent_harness() {
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        assert_dependency_absent(
            "crates/marlin-agent-test-support",
            section,
            "marlin-agent-harness",
        );
    }
}

#[test]
fn harness_consumes_test_support_and_sessions_only_from_dev_dependencies() {
    for dependency in ["marlin-agent-test-support", "marlin-agent-sessions"] {
        assert_dependency_absent("crates/marlin-agent-harness", "dependencies", dependency);
        assert_dependency_absent(
            "crates/marlin-agent-harness",
            "build-dependencies",
            dependency,
        );
        assert_dependency_present(
            "crates/marlin-agent-harness",
            "dev-dependencies",
            dependency,
        );
    }
}

#[test]
fn agent_runtime_crates_do_not_expose_native_build_readiness_surface() {
    for package in [
        "crates/marlin-agent-protocol",
        "crates/marlin-agent-core",
        "crates/marlin-agent-harness",
        "crates/marlin-agent-test-support",
    ] {
        assert_forbidden_source_terms_absent(
            package,
            &[
                "NativeGraphPolicyStrategyReadiness",
                "NativeGraphPolicyArtifact",
                "NativeGraphPolicySymbol",
                "NATIVE_GRAPH_POLICY_STRATEGY_READINESS",
                "graph_policy_strategy_readiness",
                "native_graph_policy_readiness",
            ],
        );
    }
}

fn assert_dependency_present(package_path: &str, section: &str, dependency: &str) {
    assert!(
        dependency_names(package_path, section)
            .iter()
            .any(|name| name == dependency),
        "{package_path} {section} should include {dependency}"
    );
}

fn assert_dependency_absent(package_path: &str, section: &str, dependency: &str) {
    assert!(
        dependency_names(package_path, section)
            .iter()
            .all(|name| name != dependency),
        "{package_path} {section} must not include {dependency}"
    );
}

fn dependency_names(package_path: &str, section: &str) -> Vec<String> {
    let manifest = manifest(package_path);

    manifest
        .get(section)
        .and_then(toml::Value::as_table)
        .map(|dependencies| dependencies.keys().cloned().collect())
        .unwrap_or_default()
}

fn manifest(package_path: &str) -> toml::Table {
    let manifest_path = workspace_root().join(package_path).join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", manifest_path.display()));

    toml::from_str(&manifest)
        .unwrap_or_else(|error| panic!("{} parses: {error}", manifest_path.display()))
}

fn assert_forbidden_source_terms_absent(package_path: &str, forbidden_terms: &[&str]) {
    for source_path in rust_sources_under(package_path) {
        if source_path.ends_with("tests/unit/runtime/dependency_boundary.rs") {
            continue;
        }
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", source_path.display()));
        for forbidden in forbidden_terms {
            assert!(
                !source.contains(forbidden),
                "{} must not expose build/native readiness term `{forbidden}`",
                source_path.display()
            );
        }
    }
}

fn rust_sources_under(package_path: &str) -> Vec<PathBuf> {
    let package_root = workspace_root().join(package_path);
    let mut sources = Vec::new();
    collect_rust_sources(&package_root, &mut sources);
    sources
}

fn collect_rust_sources(path: &Path, sources: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("core crate should be nested under workspace crates directory")
        .to_path_buf()
}
