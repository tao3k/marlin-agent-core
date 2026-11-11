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
