//! Task state model for `Org` TODOs and checkboxes.

use crate::OrgSourceSpan;
use serde::{Deserialize, Serialize};

/// Checkbox item inside an `Org` checklist.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgCheckbox {
    pub text: String,
    pub state: CheckboxState,
    pub source: Option<OrgSourceSpan>,
    pub marker: Option<OrgSourceSpan>,
}

impl OrgCheckbox {
    pub fn new(text: impl Into<String>, state: CheckboxState) -> Self {
        Self {
            text: text.into(),
            state,
            source: None,
            marker: None,
        }
    }
}

/// TODO state carried by an `Org` heading.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TodoState {
    Todo,
    Next,
    Wait,
    Blocked,
    Done,
    Custom(String),
}

/// Checkbox state inside an `Org` checklist.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CheckboxState {
    Open,
    Checked,
    Partial,
}
