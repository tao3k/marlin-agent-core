// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_gerbil_aot_link::discover_c_compiler;

#[test]
fn discovers_c_compiler_when_toolchain_is_available() {
    let Ok(tool) = discover_c_compiler() else {
        return;
    };

    assert!(!tool.program.as_os_str().is_empty());
}
