// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

asp_rust::asp_rust_cargo_test_gate!(mode = deny);

/// Architectural invariants ensuring `marlin-agent-search-engine` remains a
/// 100% language-agnostic, repo-agnostic, Scheme-governed native boundary.
///
/// - Language-specific AST kinds, special relations, and `providerLanguageId ->
///   languageId` aliases MUST flow dynamically from ASP providers (`GET /v1/providers`)
///   into Scheme's `*active-search-registry*`, never hardcoded in Rust tables.
/// - Hydration batch/chunk budgets MUST be projected from Scheme `HelmIndexPolicy.`
///   into the sealed [`marlin_agent_search_engine::AspHydrationPolicy`] capability
///   (no hardcoded Rust constants or `impl Default`).
mod language_agnostic_engine_boundary_guard {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
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

    fn strip_line_comments(src: &str) -> String {
        src.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn search_engine_forbids_hardcoded_language_tables_aliases_repo_paths_and_magic_budgets() {
        let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_rs_files(&src_dir, &mut files);

        let forbidden_tokens = [
            "BUILTIN_SPECIAL_RELATIONS",
            "is_builtin_search_language",
            "MAX_INSPECT_BATCH_FILES",
            "MAX_CHUNKED_HYDRATION_FILES",
            "impl Default for AspHydrationPolicy",
            "\"rust_fn\"",
            "\"rust_impl\"",
            "\"rust_struct\"",
            "\"rust_method\"",
            "\"scheme_def\"",
            "\"ts_fn\"",
            "\"py_fn\"",
            "\"julia_fn\"",
            "\"gerbil-scheme\"",
            "\"crates/marlin-agent",
            "\"scheme/search-dsl",
        ];

        let mut violations = Vec::new();
        for file in files {
            let rel = file
                .strip_prefix(&src_dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let code = strip_line_comments(&fs::read_to_string(&file).unwrap());
            for token in forbidden_tokens {
                if code.contains(token) {
                    violations.push(format!(
                        "{rel}: forbidden hardcoded language/policy token `{token}` in Rust search engine; declare in Scheme or forward from ASP provider manifests"
                    ));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Language-agnostic / Scheme-first boundary violation in marlin-agent-search-engine:\n{}",
            violations.join("\n")
        );
    }
}
