//! Domain identifiers for Scheme loop case-driver projections.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! define_loop_case_driver_string_id {
    ($type_name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $type_name(String);

        impl $type_name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $type_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl From<String> for $type_name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl From<&str> for $type_name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

define_loop_case_driver_string_id!(
    GerbilLoopCaseDriverCaseId,
    "Identifier for one Scheme config-interface loop case."
);
define_loop_case_driver_string_id!(
    GerbilLoopCaseDriverLoopProgramId,
    "Identifier for the LoopProgram emitted by a Scheme config-interface case."
);
define_loop_case_driver_string_id!(
    GerbilLoopCaseDriverProfileRef,
    "Profile reference attached to a Scheme config-interface loop case."
);
define_loop_case_driver_string_id!(
    GerbilLoopCaseDriverCapability,
    "Capability requirement attached to a Scheme config-interface loop case."
);
