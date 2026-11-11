// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_deck_runtime::DeckRuntimeMetadata;
use marlin_gerbil_scheme::{GerbilSchemeTypeId, GerbilSchemeTypedValue, GerbilSchemeValue};

#[test]
fn deck_metadata_preserves_the_scheme_version() {
    for published_version in [3, 25] {
        let projection = GerbilSchemeTypedValue::new(
            GerbilSchemeTypeId::new("marlin.deck-runtime.metadata"),
            GerbilSchemeValue::record([
                ("module-id", GerbilSchemeValue::text("deck-runtime")),
                ("version", GerbilSchemeValue::integer(published_version)),
            ]),
        );
        let metadata = DeckRuntimeMetadata::from_scheme_projection(&projection)
            .expect("map published Deck Runtime metadata");
        assert_eq!(metadata.module_id.as_str(), "deck-runtime");
        assert_eq!(metadata.version, published_version as u64);
    }
}
