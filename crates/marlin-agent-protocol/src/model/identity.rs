// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Provider and model identifier newtypes.

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

macro_rules! semantic_string {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

semantic_string!(
    /// Real model provider identifier, for example `openai`, `anthropic`, or `local`.
    ModelProviderId
);

semantic_string!(
    /// Provider-owned model name, for example `gpt-5` or `claude-opus-4-8`.
    ModelName
);

semantic_string!(
    /// Optional local alias used for configuration ergonomics.
    ModelAlias
);

semantic_string!(
    /// String passed to LiteLLM as the concrete model selector.
    LiteLlmModelId
);

impl ModelProviderId {
    pub(crate) fn heap_capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl ModelName {
    pub(crate) fn heap_capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl ModelAlias {
    pub(crate) fn heap_capacity(&self) -> usize {
        self.0.capacity()
    }
}
