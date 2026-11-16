// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "actor-runtime")]

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, LinkedGerbilProgram, RootedSchemeObject, SchemeScalar,
};
use marlin_deck_runtime::{DeckRuntimeMetadata, DeckRuntimeMetadataError};

unsafe extern "C" {
    fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
}

fn metadata_packet<'vm>(
    runtime: &'vm GerbilRuntime,
    module_id: &str,
    versions: &[u64],
) -> RootedSchemeObject<'vm> {
    let module_id = runtime.string_from_utf8(module_id).unwrap().into_object();
    let versions = runtime.numeric_vector(versions).unwrap();
    runtime
        .collection_from_objects(CollectionKind::Vector, &[&module_id, &versions])
        .unwrap()
        .into_object()
}

#[test]
fn deck_metadata_uses_rooted_scheme_values_and_rejects_malformed_receipts() {
    // SAFETY: the Actor feature links this package's declared Scheme program.
    let program = unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__deck__actor) };
    let runtime = GerbilRuntime::initialize_program_single_processor(program).unwrap();
    // Independent fixture values, not a copy of the published version.
    for version in [3, 25, u64::MAX] {
        let packet = metadata_packet(&runtime, "deck-runtime", &[version]);
        let metadata = DeckRuntimeMetadata::from_scheme_projection(packet).unwrap();
        assert_eq!(metadata.module_id.as_str(), "deck-runtime");
        assert_eq!(metadata.version, version);
    }
    let packet = metadata_packet(&runtime, "", &[25]);
    assert!(matches!(
        DeckRuntimeMetadata::from_scheme_projection(packet),
        Err(DeckRuntimeMetadataError::EmptyModuleId)
    ));
    for versions in [&[][..], &[3, 25][..]] {
        let packet = metadata_packet(&runtime, "deck-runtime", versions);
        assert!(matches!(
            DeckRuntimeMetadata::from_scheme_projection(packet),
            Err(DeckRuntimeMetadataError::VersionCount(_))
        ));
    }
    let packet = runtime
        .collection_from_objects(CollectionKind::Vector, &[])
        .unwrap()
        .into_object();
    assert!(matches!(
        DeckRuntimeMetadata::from_scheme_projection(packet),
        Err(DeckRuntimeMetadataError::FieldCount(0))
    ));
    let old_acknowledgement = runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap();
    assert!(matches!(
        DeckRuntimeMetadata::from_scheme_projection(old_acknowledgement),
        Err(DeckRuntimeMetadataError::Projection(_))
    ));
}
