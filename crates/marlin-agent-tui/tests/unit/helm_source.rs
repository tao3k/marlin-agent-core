// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{HelmAction, HelmOverlay, HelmParsedInput, HelmSourceKind};
use std::path::Path;

#[test]
fn two_stage_consult_orderless_parser_separates_async_and_local_tokens() {
    let parsed = HelmParsedInput::parse(
        "@lang:rust @doc:org @prefix:crates/ @ext:rs execute_admitted #storage !test",
    );
    assert_eq!(parsed.async_pattern, "execute_admitted");
    assert_eq!(parsed.local_tokens, vec!["storage", "!test"]);
    assert_eq!(parsed.languages, vec!["rust"]);
    assert_eq!(parsed.documents, vec!["org"]);
    assert_eq!(parsed.path_prefix.as_deref(), Some("crates/"));
    assert_eq!(parsed.extension.as_deref(), Some("rs"));
}

#[test]
fn live_helm_overlay_executes_real_asp_search_and_enforces_local_orderless_without_epoch_bump() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();

    let mut overlay = HelmOverlay::open(
        Some("@lang:rust @prefix:crates/marlin-agent-search-engine/src/ marlin_search_engine"),
        workspace_root,
        &[],
        rt.handle(),
    );
    assert!(overlay.is_searching());
    let epoch_before = overlay.epoch;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while overlay.is_searching() && std::time::Instant::now() < deadline {
        if overlay.poll() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!overlay.is_searching());
    assert!(overlay.generation_digest.is_some());
    overlay.next_source();
    assert_eq!(overlay.active_source, HelmSourceKind::Search);
    assert!(
        overlay.search_candidates.iter().any(|cand| cand
            .display
            .contains("crates/marlin-agent-search-engine/src/engine.rs")),
        "expected engine.rs hit, got {:?}",
        overlay.search_candidates
    );

    // Local Orderless filter must not bump epoch or re-trigger async search.
    overlay.insert_text(" #engine !storage", rt.handle());
    assert_eq!(overlay.epoch, epoch_before);
    assert!(!overlay.is_searching());
    assert!(!overlay.filtered_indices.is_empty());

    // Mark + Embark citation export produces structured `@path:line` citation.
    overlay.toggle_mark();
    match overlay.accept(true) {
        Some(HelmAction::InsertCitations(citations)) => {
            assert!(
                citations.contains("@crates/marlin-agent-search-engine/src/engine.rs:"),
                "unexpected citations: {citations}"
            );
        }
        other => panic!("expected InsertCitations, got {other:?}"),
    }
}
