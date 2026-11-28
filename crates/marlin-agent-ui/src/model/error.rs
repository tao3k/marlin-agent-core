// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Failures at the rooted Scheme UI boundary.

use gerbil_scheme::{NativeError, RootedCallError};

/// A rejected or failed native Scheme projection.
#[derive(Debug)]
pub enum UiProjectionError {
    Native(NativeError),
    Call(RootedCallError),
    Invalid(&'static str),
    Field {
        name: &'static str,
        source: Box<UiProjectionError>,
    },
}
impl UiProjectionError {
    pub(crate) fn at(self, name: &'static str) -> Self {
        Self::Field {
            name,
            source: Box::new(self),
        }
    }
}
impl std::fmt::Display for UiProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(error) => write!(formatter, "{error}"),
            Self::Call(error) => write!(formatter, "{error}"),
            Self::Invalid(reason) => write!(formatter, "invalid UI projection: {reason}"),
            Self::Field { name, source } => write!(formatter, "UI projection {name}: {source}"),
        }
    }
}
impl std::error::Error for UiProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Field { source, .. } => Some(source),
            _ => None,
        }
    }
}
impl From<RootedCallError> for UiProjectionError {
    fn from(error: RootedCallError) -> Self {
        Self::Call(error)
    }
}
impl From<NativeError> for UiProjectionError {
    fn from(error: NativeError) -> Self {
        Self::Native(error)
    }
}
