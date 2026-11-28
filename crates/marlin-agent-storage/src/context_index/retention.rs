// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_gitops::{
    CommittedCheckout,
    retention::{GitContextCustodian, GitContextHold, GitContextHoldId},
};

use super::{ContextIndexRead, ContextLocator, ContextLocatorKey, TursoContextIndex};
use crate::{StorageError, StorageResult};

impl TursoContextIndex {
    /// Hold an authorized locator before a caller publishes a durable reference.
    ///
    /// The caller supplies the already admitted publication and source. If the
    /// locator is superseded during acquisition, keep the hold for reconciliation
    /// and withhold the receipt. A missing receipt must never imply release.
    pub async fn retain_current_locator(
        &self,
        custodian: &GitContextCustodian,
        checkout: &CommittedCheckout,
        expected: &ContextLocator,
        publication: &GitContextHoldId,
    ) -> StorageResult<GitContextHold> {
        let key = ContextLocatorKey::for_checkout(checkout);
        self.check_current(custodian, &key, expected).await?;
        let receipt = custodian
            .retain(checkout, &expected.git, publication)
            .await
            .map_err(|error| StorageError::Backend {
                message: error.to_string(),
            })?;
        self.check_current(custodian, &key, expected).await?;
        Ok(receipt)
    }

    async fn check_current(
        &self,
        custodian: &GitContextCustodian,
        key: &ContextLocatorKey,
        expected: &ContextLocator,
    ) -> StorageResult<()> {
        let observed = self
            .lookup(custodian.instance(), key, &expected.git.path)
            .await?;
        match observed {
            ContextIndexRead::Current(current) if current == *expected => Ok(()),
            _ => Err(StorageError::Backend {
                message: "Context locator is no longer current".into(),
            }),
        }
    }
}
