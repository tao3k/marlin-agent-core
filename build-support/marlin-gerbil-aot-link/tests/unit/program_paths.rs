// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::Path;

use serde_json::json;

use super::rebase_program_paths;

#[test]
fn restored_program_resolves_package_and_sdk_paths_independently() {
    let root = std::env::temp_dir();
    let package_origin = root.join("producer/package/lib");
    let sdk_origin = root.join("producer/gerbil/lib");
    let package = root.join("consumer/package/lib");
    let sdk = root.join("consumer/gerbil/lib");
    let product = Path::new("marlin-agent-core/programs/deck-runtime");
    let runtime_source = Path::new("static/gerbil__runtime__gambit.scm");
    let package_source = Path::new("static/marlin-agent-core__deck-runtime__config.scm");
    let mut plan = json!({
        "library_dir": sdk_origin,
        "stub": package_origin.join(product).join("program__exe.scm"),
        "modules": [
            {"scm": sdk_origin.join(runtime_source), "system": true},
            {"scm": package_origin.join(package_source), "system": false, "deferred": true},
        ],
    });

    rebase_program_paths(&mut plan, product, &package, &sdk_origin, &sdk).unwrap();

    assert_eq!(plan["library_dir"], json!(sdk));
    assert_eq!(
        plan["stub"],
        json!(package.join(product).join("program__exe.scm"))
    );
    assert_eq!(plan["modules"][0]["scm"], json!(sdk.join(runtime_source)));
    assert_eq!(
        plan["modules"][1]["scm"],
        json!(package.join(package_source))
    );
    assert_eq!(plan["modules"][1]["deferred"], true);
}
