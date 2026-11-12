// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use gerbil_scheme::GerbilRuntime;
use marlin_agent_ui::{UiEvent, UiIntent, UiModule, UiOpenFacts, UiProjectionError};
use std::path::Path;

#[test]
#[ignore = "requires the compiled Marlin UI and its complete Scheme dependency load path"]
fn gerbil_resolves_and_runs_the_selected_ui_module() {
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
    let first = module.project(&initial).expect("project initial UI");
    assert_eq!(first.focus.as_str(), "composer");
    assert_eq!(first.commands().len(), 12);
    assert!(
        first
            .commands()
            .iter()
            .any(|command| command.name.as_str() == "/model" && command.accepts_arguments)
    );
    let (previous, previous_intent) = module.dispatch(&initial, UiEvent::PreviousFocus).unwrap();
    assert_eq!(
        module.project(&previous).unwrap().focus.as_str(),
        "transcript"
    );
    assert_eq!(
        previous_intent,
        UiIntent::Focus(module.project(&previous).unwrap().focus)
    );
    let (wrapped, _) = module.dispatch(&previous, UiEvent::PreviousFocus).unwrap();
    assert_eq!(module.project(&wrapped).unwrap().focus.as_str(), "commands");
    let (returned, _) = module.dispatch(&previous, UiEvent::NextFocus).unwrap();
    assert_eq!(
        module.project(&returned).unwrap().focus.as_str(),
        "composer"
    );
    let (next, intent) = module
        .dispatch(&initial, UiEvent::NextFocus)
        .expect("dispatch focus event");
    let second = module.project(&next).expect("project successor");
    assert_eq!(intent, UiIntent::Focus(second.focus.clone()));
    assert_ne!(first.focus, second.focus);
    assert_eq!(module.project(&initial).unwrap(), first);
    let refreshed = module
        .refresh_labels(&next, "new/model", "new-session")
        .unwrap();
    let updated = module.project(&refreshed).unwrap();
    assert_eq!(updated.model_label, "new/model");
    assert_eq!(updated.session_label, "new-session");
    assert_eq!(updated.focus, second.focus);
    assert_eq!(updated.regions, second.regions);
    assert_eq!(module.project(&next).unwrap(), second);
    assert!(
        module
            .refresh_labels(&next, &"x".repeat(257), "ok")
            .is_err()
    );
    let Err(UiProjectionError::Call(failure)) = module.dispatch(&initial, UiEvent::Dismiss) else {
        panic!("dismissing from the composer must fail as a Scheme call");
    };
    assert!(failure.diagnostic.is_some());
    assert_eq!(module.project(&initial).unwrap(), first);
}
