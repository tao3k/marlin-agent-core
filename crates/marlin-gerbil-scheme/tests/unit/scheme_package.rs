// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use gerbil_scheme::GerbilRuntime;
use marlin_gerbil_scheme::gerbil_package_root;

#[test]
#[ignore = "requires the Gerbil SDK and the installed Scheme package dependencies"]
fn gerbil_builds_and_tests_the_marlin_package_in_process() {
    let directory = gerbil_package_root()
        .canonicalize()
        .expect("Scheme package");
    let directory = directory.to_str().expect("UTF-8 package path");
    let runtime = GerbilRuntime::initialize().expect("initialize Gerbil runtime");

    assert_eq!(
        runtime
            .build_package(directory)
            .expect("Gerbil Builder call"),
        0
    );
    assert_eq!(
        runtime
            .test_package(directory, "t/all-test.ss")
            .expect("Gerbil Test call"),
        0
    );
}

#[test]
#[ignore = "requires the Gerbil SDK and the installed Scheme package dependencies"]
fn gerbil_tests_the_ui_workbench_in_process() {
    let directory = gerbil_package_root()
        .canonicalize()
        .expect("Scheme package");
    let directory = directory.to_str().expect("UTF-8 package path");
    let runtime = GerbilRuntime::initialize().expect("initialize Gerbil runtime");
    assert_eq!(
        runtime
            .test_package(directory, "t/ui-native-test.ss")
            .expect("Gerbil Test call"),
        0
    );
}

#[test]
#[ignore = "requires the compiled Marlin UI and its complete Scheme dependency load path"]
fn gerbil_resolves_and_runs_the_selected_ui_module() {
    use marlin_gerbil_scheme::ui::{UiEvent, UiIntent, UiModule, UiOpenFacts, UiProjectionError};

    let directory = gerbil_package_root()
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
    let (next, intent) = module
        .dispatch(&initial, UiEvent::NextFocus)
        .expect("dispatch focus event");
    let second = module.project(&next).expect("project successor");
    assert_eq!(intent, UiIntent::Focus(second.focus.clone()));
    assert_ne!(first.focus, second.focus);
    assert_eq!(module.project(&initial).unwrap(), first);
    let Err(UiProjectionError::Call(failure)) = module.dispatch(&initial, UiEvent::Dismiss) else {
        panic!("dismissing from the composer must fail as a Scheme call");
    };
    assert!(failure.diagnostic.is_some());
    assert_eq!(module.project(&initial).unwrap(), first);
}

#[test]
#[ignore = "requires the compiled UI ABI driver and Gerbil package dependencies"]
fn gerbil_tests_the_ui_c_abi_in_process() {
    let directory = gerbil_package_root()
        .canonicalize()
        .expect("Scheme package");
    let directory = directory.to_str().expect("UTF-8 package path");
    let runtime = GerbilRuntime::initialize().expect("initialize Gerbil runtime");
    assert_eq!(
        runtime
            .test_package(directory, "t/ui-c-abi-test.ss")
            .expect("Gerbil Test call"),
        0
    );
}
