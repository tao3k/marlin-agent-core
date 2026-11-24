// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Private Git references and durable claims owned by one enrolled repository.

mod custody;
mod refs;

pub use custody::{
    GitContextCustodian, GitContextDisposal, GitContextHold, GitContextHoldId, GitRetentionError,
};
