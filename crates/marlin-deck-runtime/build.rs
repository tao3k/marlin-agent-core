// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

fn main() {
    #[cfg(feature = "actor-runtime")]
    marlin_gerbil_aot_link::emit_deck_actor_link_directives();
}
