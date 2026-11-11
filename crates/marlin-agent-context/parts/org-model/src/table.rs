// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! `Org` table model used for metrics and structured records.

use serde::{Deserialize, Serialize};

/// Table with optional name, header row, and body rows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgTable {
    pub name: Option<String>,
    pub header: Vec<String>,
    pub rows: Vec<OrgTableRow>,
}

/// Row of table cell strings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgTableRow {
    pub cells: Vec<String>,
}
