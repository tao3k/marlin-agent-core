// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rust projection of metadata published by the Scheme Deck Runtime Module.

use serde::{Deserialize, Serialize};

#[cfg(feature = "actor-runtime")]
use gerbil_scheme::{
    CollectionKind, NativeError as SchemeVmError, NumericBuffer, RootedSchemeObject,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeckRuntimeModuleId(pub(crate) String);

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
    /// Decode the Scheme startup receipt through the SDK's strong-root API.
    ///
    /// Scheme supplies the module ID and version. The receipt is a vector of
    /// its string ID and a one-element u64vector; no version table is kept here.
    #[cfg(feature = "actor-runtime")]
    pub fn from_scheme_projection(
        projection: RootedSchemeObject<'_>,
    ) -> Result<Self, DeckRuntimeMetadataError> {
        let fields = projection.into_collection(CollectionKind::Vector)?;
        if fields.len() != 2 {
            return Err(DeckRuntimeMetadataError::FieldCount(fields.len()));
        }
        let module_id = fields
            .element(0)?
            .into_string()?
            .to_string()
            .into_result()?;
        if module_id.is_empty() {
            return Err(DeckRuntimeMetadataError::EmptyModuleId);
        }
        let mut version = NumericBuffer::<u64>::default();
        fields.element(1)?.as_ref().copy_numeric(&mut version)?;
        let [version] = version.as_slice() else {
            return Err(DeckRuntimeMetadataError::VersionCount(
                version.as_slice().len(),
            ));
        };
        Ok(Self {
            module_id: DeckRuntimeModuleId(module_id),
            version: *version,
        })
    }
}

#[cfg(feature = "actor-runtime")]
#[derive(Debug, thiserror::Error)]
pub enum DeckRuntimeMetadataError {
    #[error("Deck metadata requires two fields; got {0}")]
    FieldCount(usize),
    #[error("Deck metadata requires a nonempty module ID")]
    EmptyModuleId,
    #[error("Deck metadata requires one version; got {0}")]
    VersionCount(usize),
    #[error("Deck metadata VM projection failed: {0}")]
    Projection(#[from] SchemeVmError),
}
