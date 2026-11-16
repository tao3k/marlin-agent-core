// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! `Org` block model used by workspace records.

use serde::{Deserialize, Serialize};

/// Block metadata without forcing raw block body into agent context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgBlock {
    pub kind: BlockKind,
    pub name: Option<String>,
    pub language: Option<String>,
    pub body_hash: Option<String>,
}

/// Supported `Org` block classes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    Source,
    Example,
    Quote,
    Export,
    Custom(String),
}
