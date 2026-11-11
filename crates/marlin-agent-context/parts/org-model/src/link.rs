// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! `Org` link model used for evidence and cross references.

use serde::{Deserialize, Serialize};

/// Link target with optional human-readable description.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgLink {
    pub kind: LinkKind,
    pub target: String,
    pub description: Option<String>,
}

/// Link namespace understood by the workspace model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LinkKind {
    Id,
    File,
    Url,
    Custom(String),
}
