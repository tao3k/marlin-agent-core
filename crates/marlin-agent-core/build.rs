// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

fn main() {
    if cfg!(feature = "brief-to-draft-vm-test") {
        marlin_gerbil_aot_link::emit_brief_to_draft_link_directives();
    }
}
