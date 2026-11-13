// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::write_with_authority;
use std::future::{pending, ready};

#[tokio::test]
async fn cancelled_write_never_restores_authority_without_confirmation() {
    let mut owned = true;
    {
        let mut write = Box::pin(write_with_authority(
            &mut owned,
            pending::<Result<(), ()>>(),
        ));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut write)
                .await
                .is_err()
        );
    }
    assert!(!owned);
}

#[tokio::test]
async fn only_a_confirmed_write_restores_authority() {
    let mut owned = true;
    assert!(
        write_with_authority(&mut owned, ready(Err::<(), _>("failed")))
            .await
            .is_err()
    );
    assert!(!owned);
    assert!(
        write_with_authority(&mut owned, ready(Ok::<(), ()>(())))
            .await
            .is_ok()
    );
    assert!(owned);
}
