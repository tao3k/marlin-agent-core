// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rust projection of metadata published by the Scheme Deck Runtime Module.

use serde::{Deserialize, Serialize};

use marlin_gerbil_scheme::{GerbilSchemeTypeDecodeError, GerbilSchemeTypedValue};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeckRuntimeModuleId(String);

impl DeckRuntimeModuleId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeckRuntimeMetadata {
    pub module_id: DeckRuntimeModuleId,
    pub version: u64,
}

impl DeckRuntimeMetadata {
    /// Map the in-memory typed Scheme projection into the Rust metadata record.
    pub fn from_scheme_projection(
        projection: &GerbilSchemeTypedValue,
    ) -> Result<Self, GerbilSchemeTypeDecodeError> {
        projection.decode_value()
    }
}
