// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "../../benches/support/scope_identity.rs"]
mod identity;

#[test]
fn fixture_identity_matches_a_frozen_known_answer() {
    assert_eq!(
        identity::expected_digest(0),
        [
            0x01, 0x58, 0x5e, 0x46, 0x6e, 0xfc, 0x6e, 0x84, 0x6e, 0xd4, 0xde, 0x3d, 0x6f, 0x85,
            0x40, 0xe7, 0x53, 0xc8, 0xec, 0x05, 0xe1, 0xc8, 0x19, 0x9a, 0x2c, 0x8d, 0x63, 0xfd,
            0x18, 0x6b, 0x9e, 0x21,
        ]
    );
}

#[test]
fn receipt_oracle_rejects_substitution_duplicates_and_omissions() {
    let expected = [identity::expected_digest(0), identity::expected_digest(1)];
    assert!(
        identity::verify(
            &expected,
            [(1, &expected[1]), (0, &expected[0])].into_iter()
        )
        .is_ok()
    );
    let mut changed = expected[0];
    changed[0] ^= 1;
    for observed in [
        [(0, &changed), (1, &expected[1])],
        [(0, &expected[1]), (1, &expected[0])],
        [(0, &expected[0]), (0, &expected[0])],
        [(0, &expected[0]), (2, &expected[1])],
    ] {
        assert!(identity::verify(&expected, observed.into_iter()).is_err());
    }
    assert!(identity::verify(&expected, [(0, &expected[0])].into_iter()).is_err());
}
