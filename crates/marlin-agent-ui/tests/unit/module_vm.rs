// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(not(feature = "linked-ui"))]

use gerbil_scheme::GerbilRuntime;
use marlin_agent_protocol::HumanReviewKind;
use marlin_agent_ui::{UiEvent, UiIntent, UiModule, UiOpenFacts, UiProjectionError};
use std::path::Path;

// Scheme tests own command order, labels, profiles, focus rules, and review
// choices. This smoke test exercises typed calls across the live VM boundary;
// synthetic projection tests cover malformed ABI values.
#[test]
#[ignore = "requires the compiled Marlin UI and its complete Scheme dependency load path"]
fn selected_ui_module_crosses_the_live_vm_boundary() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("Scheme package");
    let runtime = GerbilRuntime::initialize().expect("initialize Gerbil runtime");
    let module = UiModule::resolve(&runtime, directory.to_str().expect("UTF-8 package path"))
        .expect("resolve selected UI publication and procedures");
    let facts = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &[],
        extensions: &[],
        method_bundles: &[],
    };
    let initial = module.open(&facts).expect("open selected UI");
    let projected = module.project(&initial).expect("decode selected UI");
    assert!(!projected.regions.is_empty());
    assert!(!module
        .review_menu(&initial, HumanReviewKind::PermissionEscalation)
        .expect("decode selected review menu")
        .choices
        .is_empty());

    let (successor, intent) = module
        .dispatch(&initial, UiEvent::NextFocus)
        .expect("dispatch through the selected UI");
    let next = module.project(&successor).expect("decode successor");
    assert!(matches!(intent, UiIntent::Focus(ref id) if id == &next.focus));
    assert_eq!(module.project(&initial).unwrap(), projected);

    let Err(UiProjectionError::Call(failure)) = module.dispatch(&initial, UiEvent::Dismiss) else {
        panic!("Scheme rejection must cross as a typed call failure");
    };
    assert!(failure.diagnostic.is_some());
}
